use crate::{DependencyGroupSpecifier, PyProjectToml};
use indexmap::IndexMap;
use pep508_rs::Requirement;
use thiserror::Error;

/// Resolves a single dependency group or extra.
pub fn resolve_group<'a, T: DependencyEntry>(
    groups: &'a IndexMap<String, Vec<T>>,
    group: &'a str,
    resolved: &mut IndexMap<String, Vec<Requirement>>,
    parents: &mut Vec<&'a str>,
    project_name: Option<&'a str>,
) -> Result<(), RecursionResolutionError> {
    // If the group has already been resolved, exit early
    if resolved.get(group).is_some() {
        return Ok(());
    }
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
                    resolve_group(groups, group, resolved, parents, project_name)?;
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

/// A trait that defines how to parse a recursion item.
pub trait DependencyEntry {
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
    #[error(
        "Group `{0}` is defined in both `project.optional-dependencies` and `dependency-groups`"
    )]
    NameCollision(String),
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

type ResolvedDependencies = IndexMap<String, Vec<Requirement>>;

impl PyProjectToml {
    /// Resolve the optional dependencies and dependency groups into flat lists of requirements.
    ///
    /// This function will recursively resolve all optional dependency groups and dependency groups,
    /// including those that reference other groups. It will return an error if
    ///  - there is a cycle in the groups,
    ///  - a group references another group that does not exist, or
    ///  - there is a name collision between optional dependencies and dependency groups.
    pub fn resolve(
        &self,
    ) -> Result<
        (Option<ResolvedDependencies>, Option<ResolvedDependencies>),
        RecursionResolutionError,
    > {
        let self_reference_name = self.project.as_ref().map(|p| p.name.as_str());
        let optional_dependencies = self
            .project
            .as_ref()
            .and_then(|p| p.optional_dependencies.as_ref());

        // Check for collisions between optional dependencies and dependency groups.
        if let (Some(optional_dependencies), Some(dependency_groups)) =
            (optional_dependencies, self.dependency_groups.as_ref())
        {
            for group in optional_dependencies.keys() {
                if dependency_groups.contains_key(group) {
                    return Err(RecursionResolutionError::NameCollision(group.clone()));
                }
            }
        }

        // Resolve optional dependencies
        let mut resolved_optional_dependencies = IndexMap::new();
        if let Some(optional_dependencies_map) = optional_dependencies {
            for group in optional_dependencies_map.keys() {
                resolve_group(
                    optional_dependencies_map,
                    group,
                    &mut resolved_optional_dependencies,
                    &mut Vec::new(),
                    self_reference_name,
                )?;
            }
        }

        // Resolve dependency groups, which may reference optional dependencies.
        // Start with a clone of resolved_optional_dependencies so that dependency groups can reference them.
        let mut resolved_dependency_groups = resolved_optional_dependencies.clone();
        if let Some(dependency_groups_map) = self.dependency_groups.as_ref() {
            for group in dependency_groups_map.keys() {
                resolve_group(
                    dependency_groups_map,
                    group,
                    &mut resolved_dependency_groups,
                    &mut Vec::new(),
                    self_reference_name,
                )?;
            }
        }

        // Remove optional dependency groups from the resolved dependency groups.
        if let Some(optional_dependencies_map) = optional_dependencies {
            for key in optional_dependencies_map.keys() {
                resolved_dependency_groups.shift_remove(key);
            }
        }

        Ok((
            if resolved_optional_dependencies.is_empty() {
                None
            } else {
                Some(resolved_optional_dependencies)
            },
            if resolved_dependency_groups.is_empty() {
                None
            } else {
                Some(resolved_dependency_groups)
            },
        ))
    }
}

impl DependencyEntry for Requirement {
    fn parse<'a>(&'a self, name: Option<&str>) -> Item<'a> {
        if name.map(|n| n == self.name.to_string()).unwrap_or(false) {
            Item::Groups(self.extras.iter().map(|extra| extra.as_ref()).collect())
        } else {
            Item::Requirement(self.clone())
        }
    }
    fn table_name() -> String {
        "project.optional-dependencies".to_string()
    }
    fn group_name() -> String {
        "optional dependency group".to_string()
    }
}

impl DependencyEntry for DependencyGroupSpecifier {
    fn parse<'a>(&'a self, name: Option<&str>) -> Item<'a> {
        match self {
            DependencyGroupSpecifier::String(requirement) => {
                if name
                    .map(|n| n == requirement.name.to_string())
                    .unwrap_or(false)
                {
                    Item::Groups(
                        requirement
                            .extras
                            .iter()
                            .map(|extra| extra.as_ref())
                            .collect(),
                    )
                } else {
                    Item::Requirement(requirement.clone())
                }
            }
            DependencyGroupSpecifier::Table {
                include_group: group,
            } => Item::Groups(vec![group]),
        }
    }
    fn table_name() -> String {
        "dependency-groups".to_string()
    }
    fn group_name() -> String {
        "dependency group".to_string()
    }
}

#[cfg(test)]
mod tests {
    use pep508_rs::Requirement;
    use std::str::FromStr;

    use crate::PyProjectToml;

    #[test]
    fn test_parse_pyproject_toml_optional_dependencies_resolve() {
        let source = r#"[project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = ["spam[alpha]"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let (optional_dependencies, _) = project_toml.resolve().unwrap();

        assert_eq!(
            optional_dependencies.unwrap()["iota"],
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
        assert_eq!(
            project_toml.resolve().unwrap_err().to_string(),
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
        assert_eq!(
            project_toml.resolve().unwrap_err().to_string(),
            String::from("Failed to find optional dependency group `alpha` included by `iota`")
        )
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_resolve() {
        let source = r#"
            [dependency-groups]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = [{include-group = "alpha"}]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let (_, dependency_groups) = project_toml.resolve().unwrap();

        assert_eq!(
            dependency_groups.unwrap()["iota"],
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
        assert_eq!(
            project_toml.resolve().unwrap_err().to_string(),
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
        assert_eq!(
            project_toml.resolve().unwrap_err().to_string(),
            String::from("Failed to find dependency group `alpha` included by `iota`")
        )
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_with_optional_dependencies() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            test = ["pytest"]

            [dependency-groups]
            dev = ["spam[test]"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let (_, dependency_groups) = project_toml.resolve().unwrap();
        assert_eq!(
            dependency_groups.unwrap()["dev"],
            vec![Requirement::from_str("pytest").unwrap()]
        );
    }

    #[test]
    fn test_name_collision() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            dev = ["pytest"]

            [dependency-groups]
            dev = ["ruff"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let err = project_toml.resolve().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Group `dev` is defined in both `project.optional-dependencies` and `dependency-groups`"
        );
    }

    #[test]
    fn test_optional_dependencies_are_not_dependency_groups() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            test = ["pytest"]

            [dependency-groups]
            dev = ["spam[test]"]
        "#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let (optional_dependencies, dependency_groups) = project_toml.resolve().unwrap();
        assert!(optional_dependencies.unwrap().contains_key("test"));
        assert!(!dependency_groups.as_ref().unwrap().contains_key("test"));
        assert!(dependency_groups.unwrap().contains_key("dev"));
    }
}
