use indexmap::IndexMap;
use pep508_rs::Requirement;
use thiserror::Error;

use crate::{DependencyGroupSpecifier, DependencyGroups};

#[derive(Debug, Error)]
pub enum Pep735Error {
    #[error("Failed to find group `{0}` included by `{1}`")]
    GroupNotFound(String, String),
    #[error("Detected a cycle in `dependency-groups`: {0}")]
    DependencyGroupCycle(Cycle),
}

/// A cycle in the `dependency-groups` table.
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

impl DependencyGroups {
    /// Resolve dependency groups (which may contain references to other groups) into concrete
    /// lists of requirements.
    pub fn resolve(&self) -> Result<IndexMap<String, Vec<Requirement>>, Pep735Error> {
        // Helper function to resolves a single group
        fn resolve_single<'a>(
            groups: &'a DependencyGroups,
            group: &'a str,
            resolved: &mut IndexMap<String, Vec<Requirement>>,
            parents: &mut Vec<&'a str>,
        ) -> Result<(), Pep735Error> {
            let Some(specifiers) = groups.get(group) else {
                // If the group included in another group does not exist, return an error
                let parent = parents.iter().last().expect("should have a parent");
                return Err(Pep735Error::GroupNotFound(
                    group.to_string(),
                    parent.to_string(),
                ));
            };
            // If there is a cycle in dependency groups, return an error
            if parents.contains(&group) {
                return Err(Pep735Error::DependencyGroupCycle(Cycle(
                    parents.iter().map(|s| s.to_string()).collect(),
                )));
            }
            // If the dependency group has already been resolved, exit early
            if resolved.get(group).is_some() {
                return Ok(());
            }
            // Otherwise, perform recursion, as required, on the dependency group's specifiers
            parents.push(group);
            let mut requirements = Vec::with_capacity(specifiers.len());
            for spec in specifiers.iter() {
                match spec {
                    // It's a requirement. Just add it to the Vec of resolved requirements
                    DependencyGroupSpecifier::String(requirement) => {
                        requirements.push(requirement.clone())
                    }
                    // It's a reference to another group. Recurse into it
                    DependencyGroupSpecifier::Table { include_group } => {
                        resolve_single(groups, include_group, resolved, parents)?;
                        requirements
                            .extend(resolved.get(include_group).into_iter().flatten().cloned());
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
            resolve_single(self, group, &mut resolved, &mut Vec::new())?;
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
        let source = r#"[dependency-groups]
alpha = ["beta", "gamma", "delta"]
epsilon = ["eta<2.0", "theta==2024.09.01"]
iota = [{include-group = "alpha"}]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();

        assert_eq!(
            dependency_groups.resolve().unwrap()["iota"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_cycle() {
        let source = r#"[dependency-groups]
alpha = [{include-group = "iota"}]
iota = [{include-group = "alpha"}]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();
        assert_eq!(
            dependency_groups.resolve().unwrap_err().to_string(),
            String::from("Detected a cycle in `dependency-groups`: `alpha` -> `iota` -> `alpha`")
        )
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_missing_include() {
        let source = r#"[dependency-groups]
iota = [{include-group = "alpha"}]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();
        assert_eq!(
            dependency_groups.resolve().unwrap_err().to_string(),
            String::from("Failed to find group `alpha` included by `iota`")
        )
    }
}
