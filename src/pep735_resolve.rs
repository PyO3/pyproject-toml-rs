use crate::has_recursion::{HasRecursion, Item, RecursionItem};

use crate::{DependencyGroupSpecifier, DependencyGroups};

impl HasRecursion<DependencyGroupSpecifier> for DependencyGroups {}

impl RecursionItem for DependencyGroupSpecifier {
    fn parse(&self, _name: Option<&str>) -> Item {
        match self {
            DependencyGroupSpecifier::String(requirement) => Item::Requirement(requirement.clone()),
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
            String::from("Failed to find dependency group `alpha` included by `iota`")
        )
    }
}
