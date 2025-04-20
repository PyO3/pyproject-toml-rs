use indexmap::IndexMap;
use pep508_rs::Requirement;
use std::ops::Deref;
use thiserror::Error;

/// A trait that resolves recursions for groups of requirements that can be mapped to `IndexMap<String, Vec<T>>`
/// where T is a type that can be mapped to either a Requirement or a reference to other groups of requirements.
pub trait HasRecursion<T>: Deref<Target = IndexMap<String, Vec<T>>>
where
    T: RecursionItem,
{
    /// Resolve the groups into lists of requirements.
    ///
    /// This function will recursively resolve all groups, including those that
    /// reference other groups. It will return an error if there is a cycle in the
    /// groups or if a group references another group that does not exist.
    fn resolve(&self) -> Result<IndexMap<String, Vec<Requirement>>, RecursionResolutionError> {
        self.resolve_all(None)
    }

    /// Resolves the groups of requirements into flat lists of requirements.
    fn resolve_all(
        &self,
        name: Option<&str>,
    ) -> Result<IndexMap<String, Vec<Requirement>>, RecursionResolutionError> {
        // Helper function to resolve a single group
        fn resolve_single<'a, T: RecursionItem>(
            groups: &'a IndexMap<String, Vec<T>>,
            group: &'a str,
            resolved: &mut IndexMap<String, Vec<Requirement>>,
            parents: &mut Vec<&'a str>,
            name: Option<&'a str>,
        ) -> Result<(), RecursionResolutionError> {
            let Some(items) = groups.get(group) else {
                // If the group included in another group does not exist, return an error
                let parent = parents.iter().last().expect("should have a parent");
                return Err(RecursionResolutionError::GroupNotFound(
                    T::group_name(),
                    group.to_string(),
                    parent.to_string(),
                ));
            };
            // If there is a cycle in dependency groups, return an error
            if parents.contains(&group) {
                return Err(RecursionResolutionError::DependencyGroupCycle(
                    T::table_name(),
                    Cycle(parents.iter().map(|s| s.to_string()).collect()),
                ));
            }
            // If the group has already been resolved, exit early
            if resolved.get(group).is_some() {
                return Ok(());
            }
            // Otherwise, perform recursion, as required, on the dependency group's specifiers
            parents.push(group);
            let mut requirements = Vec::with_capacity(items.len());
            for spec in items.iter() {
                match spec.parse(name) {
                    // It's a requirement. Just add it to the Vec of resolved requirements
                    Item::Requirement(requirement) => requirements.push(requirement.clone()),
                    // It's a reference to other groups. Recurse into them
                    Item::Groups(inner_groups) => {
                        for group in inner_groups {
                            resolve_single(groups, group, resolved, parents, name)?;
                            requirements.extend(resolved.get(group).into_iter().flatten().cloned());
                        }
                    }
                }
            }
            // Add the resolved group to IndexMap
            resolved.insert(group.to_string(), requirements.clone());
            parents.pop();
            Ok(())
        }

        let mut resolved = IndexMap::new();
        for group in self.keys() {
            resolve_single(self, group, &mut resolved, &mut Vec::new(), name)?;
        }
        Ok(resolved)
    }
}
/// A trait that defines how to parse a recursion item.
pub trait RecursionItem {
    /// Parse the item into a requirement or a reference to other groups.
    fn parse<'a>(&'a self, name: Option<&str>) -> Item<'a>;
    /// The name of the group in the TOML file.
    fn group_name() -> String;
    /// The name of the table in the TOML file.
    fn table_name() -> String;
}

pub enum Item<'a> {
    Requirement(Requirement),
    Groups(Vec<&'a str>),
}

#[derive(Debug, Error)]
pub enum RecursionResolutionError {
    #[error("Failed to find {0} `{1}` included by `{2}`")]
    GroupNotFound(String, String, String),
    #[error("Detected a cycle in `{0}`: {1}")]
    DependencyGroupCycle(String, Cycle),
}

/// A cycle in the recursion.
#[derive(Debug)]
pub struct Cycle(Vec<String>);

/// Display a cycle, e.g., `a -> b -> c -> a`.
impl std::fmt::Display for Cycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [first, rest @ ..] = self.0.as_slice() else {
            return Ok(());
        };
        write!(f, "`{first}`")?;
        for group in rest {
            write!(f, " -> `{group}`")?;
        }
        write!(f, " -> `{first}`")?;
        Ok(())
    }
}
