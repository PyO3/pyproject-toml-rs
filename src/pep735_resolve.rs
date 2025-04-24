use crate::{DependencyGroupSpecifier, DependencyGroups, OptionalDependencies};
use indexmap::IndexMap;
use pep508_rs::Requirement;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecursionResolutionError {
    #[error("Failed to find {0} `{1}` included by `{2}`")]
    GroupNotFound(String, String, String),
    #[error("Detected a cycle in `{0}`: {1}")]
    DependencyGroupCycle(String, Cycle),
}

/// A cycle in the recursion.
#[derive(Debug)]
pub struct Cycle(pub Vec<String>);

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

pub(crate) enum Item<'a> {
    Requirement(Requirement),
    Groups(Vec<&'a str>),
}

/// A trait that defines how to parse a recursion item.
pub(crate) trait DependencyEntry {
    /// Parse the item into a requirement or a reference to other groups.
    ///
    /// If the name is `None`, self-referential extras are not resolved.
    fn parse(&self, name: Option<&str>) -> Item;
    /// The name of the group in `pyproject.toml`.
    fn group_name() -> String;
    /// The name of the table in `pyproject.toml`.
    fn table_name() -> String;
}

impl DependencyEntry for DependencyGroupSpecifier {
    fn parse(&self, _name: Option<&str>) -> Item {
        match self {
            DependencyGroupSpecifier::String(requirement) => Item::Requirement(requirement.clone()),
            DependencyGroupSpecifier::Table {
                include_group: group,
            } => Item::Groups(vec![group]),
        }
    }

    fn group_name() -> String {
        "dependency group".to_string()
    }

    fn table_name() -> String {
        "dependency-groups".to_string()
    }
}

impl DependencyEntry for Requirement {
    fn parse(&self, name: Option<&str>) -> Item {
        if name
            .map(|name| name == self.name.to_string())
            .unwrap_or(false)
        {
            Item::Groups(self.extras.iter().map(|extra| extra.as_ref()).collect())
        } else {
            Item::Requirement(self.clone())
        }
    }

    fn group_name() -> String {
        "optional dependency group".to_string()
    }

    fn table_name() -> String {
        "project.optional-dependencies".to_string()
    }
}

/// Resolve a single dependency group or extra.
fn resolve_group<'a, T: DependencyEntry>(
    group: &'a str,
    groups: &'a IndexMap<String, Vec<T>>,
    resolved: &mut IndexMap<String, Vec<Requirement>>,
    parents: &mut Vec<&'a str>,
    project_name: Option<&'a str>,
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
        match spec.parse(project_name) {
            // It's a requirement. Just add it to the Vec of resolved requirements
            Item::Requirement(requirement) => requirements.push(requirement.clone()),
            // It's a reference to other groups. Recurse into them
            Item::Groups(inner_groups) => {
                for group in inner_groups {
                    resolve_group(group, groups, resolved, parents, project_name)?;
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

impl DependencyGroups {
    /// Resolve include groups and self-references in dependency groups.
    ///
    /// If the project name is `None`, self-referential extras are not resolved.
    ///
    /// Errors if there is a cycle in the groups or if a group references another group that does
    /// not exist.
    pub fn resolve(
        &self,
        project_name: Option<&str>,
    ) -> Result<IndexMap<String, Vec<Requirement>>, RecursionResolutionError> {
        let mut resolved = IndexMap::new();
        for group in self.keys() {
            resolve_group(group, self, &mut resolved, &mut Vec::new(), project_name)?;
        }
        Ok(resolved)
    }
}

impl OptionalDependencies {
    /// Resolve self-references in optional dependencies.
    ///
    /// If the project name is `None`, self-referential extras are not resolved, returning the list
    /// unchanged.
    ///
    /// `project_name` is the name of the project itself, which is used to identify self-references
    /// in the optional dependencies. If None, self-references will be treated as normal dependencies.
    pub fn resolve(
        &self,
        project_name: Option<&str>,
    ) -> Result<IndexMap<String, Vec<Requirement>>, RecursionResolutionError> {
        let mut resolved = IndexMap::new();
        for group in self.keys() {
            resolve_group(group, self, &mut resolved, &mut Vec::new(), project_name)?;
        }
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use pep508_rs::Requirement;
    use std::str::FromStr;

    use crate::PyProjectToml;

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_resolve() {
        let source = r#"
            [dependency-groups]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = [{include-group = "alpha"}]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();

        assert_eq!(
            dependency_groups.resolve(None).unwrap()["iota"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_cycle() {
        let source = r#"
            [dependency-groups]
            alpha = [{include-group = "iota"}]
            iota = [{include-group = "alpha"}]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();
        assert_eq!(
            dependency_groups.resolve(None).unwrap_err().to_string(),
            String::from("Detected a cycle in `dependency-groups`: `alpha` -> `iota` -> `alpha`")
        )
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_missing_include() {
        let source = r#"
            [dependency-groups]
            iota = [{include-group = "alpha"}]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();
        assert_eq!(
            dependency_groups.resolve(None).unwrap_err().to_string(),
            String::from("Failed to find dependency group `alpha` included by `iota`")
        )
    }

    #[test]
    fn test_parse_pyproject_toml_optional_dependencies_resolve() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = ["spam[alpha]"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let optional_dependencies = project_toml
            .project
            .as_ref()
            .unwrap()
            .optional_dependencies
            .as_ref()
            .unwrap();

        assert_eq!(
            optional_dependencies.resolve(Some("spam")).unwrap()["iota"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }

    #[test]
    fn test_parse_pyproject_toml_optional_dependencies_cycle() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["spam[iota]"]
            iota = ["spam[alpha]"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let optional_dependencies = project_toml
            .project
            .as_ref()
            .unwrap()
            .optional_dependencies
            .as_ref()
            .unwrap();
        assert_eq!(
            optional_dependencies
                .resolve(Some("spam"))
                .unwrap_err()
                .to_string(),
            String::from(
                "Detected a cycle in `project.optional-dependencies`: `alpha` -> `iota` -> `alpha`"
            )
        )
    }

    #[test]
    fn test_parse_pyproject_toml_optional_dependencies_missing_include() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            iota = ["spam[alpha]"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let optional_dependencies = project_toml
            .project
            .as_ref()
            .unwrap()
            .optional_dependencies
            .as_ref()
            .unwrap();
        assert_eq!(
            optional_dependencies
                .resolve(Some("spam"))
                .unwrap_err()
                .to_string(),
            String::from("Failed to find optional dependency group `alpha` included by `iota`")
        )
    }

    #[test]
    fn test_parse_pyproject_toml_both_resolve() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = ["spam[alpha]"]

            [dependency-groups]
            mu = ["spam[iota]"]
            nu = [{include-group = "mu"}]
            xi = ["lambda"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();

        assert_eq!(
            dependency_groups.resolve(Some("spam")).unwrap()["nu"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }
}
