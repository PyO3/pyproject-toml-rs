use indexmap::IndexMap;
use pep508_rs::Requirement;

use crate::has_recursion::{HasRecursion, Item, RecursionItem, RecursionResolutionError};
use crate::{DependencyGroupSpecifier, DependencyGroups, OptionalDependencies, PyProjectToml};

impl PyProjectToml {
    /// Resolve the optional dependencies and dependency groups into flat lists of requirements.
    ///
    /// This function will recursively resolve all optional dependency groups and dependency groups,
    /// including those that reference other groups. It will return an error if there is a cycle
    /// in the groups or if a group references another group that does not exist.
    pub fn resolve(
        &self,
    ) -> Result<
        (
            Option<IndexMap<String, Vec<Requirement>>>,
            Option<IndexMap<String, Vec<Requirement>>>,
        ),
        RecursionResolutionError,
    > {
        let self_reference_name = self.project.as_ref().map(|p| p.name.as_str());

        // Resolve optional dependencies first, as they may be referenced by dependency groups.
        let resolved_optional_dependencies = self
            .project
            .as_ref()
            .and_then(|project| project.optional_dependencies.as_ref())
            .map(|optional_dependencies| {
                optional_dependencies.resolve_all(self_reference_name, None)
            })
            .transpose()?;

        // Resolve dependency groups, which may reference optional dependencies.
        let resolved_dependency_groups = self
            .dependency_groups
            .as_ref()
            .map(|dependency_groups| {
                dependency_groups
                    .resolve_all(self_reference_name, resolved_optional_dependencies.clone())
            })
            .transpose()?;

        Ok((resolved_optional_dependencies, resolved_dependency_groups))
    }
}

impl HasRecursion<Requirement> for OptionalDependencies {}

impl HasRecursion<DependencyGroupSpecifier> for DependencyGroups {}

impl RecursionItem for Requirement {
    fn parse(&self, name: Option<&str>) -> Item {
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

impl RecursionItem for DependencyGroupSpecifier {
    fn parse(&self, name: Option<&str>) -> Item {
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
        let source = r#"[project]
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
        let source = r#"[project]
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
        let source = r#"[dependency-groups]
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
        let source = r#"[dependency-groups]
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
        let source = r#"[dependency-groups]
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
        let source = r#"[project]
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
}
