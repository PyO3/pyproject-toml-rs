use crate::{
    has_recursion::{HasRecursion, Item, RecursionItem, RecursionResolutionError},
    OptionalDependencies,
};
use indexmap::IndexMap;
use pep508_rs::Requirement;

impl HasRecursion<Requirement> for OptionalDependencies {
    fn resolve(&self) -> Result<IndexMap<String, Vec<Requirement>>, RecursionResolutionError> {
        self.resolve_all(self.self_reference_name.as_deref())
    }
}

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
#[cfg(test)]
mod tests {
    use pep508_rs::Requirement;
    use std::str::FromStr;

    use crate::PyProjectToml;

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_resolve() {
        let source = r#"[project]
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
            optional_dependencies.resolve().unwrap()["iota"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_cycle() {
        let source = r#"[project]
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
            optional_dependencies.resolve().unwrap_err().to_string(),
            String::from(
                "Detected a cycle in `project.optional-dependencies`: `alpha` -> `iota` -> `alpha`"
            )
        )
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups_missing_include() {
        let source = r#"[project]
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
            optional_dependencies.resolve().unwrap_err().to_string(),
            String::from("Failed to find optional dependency group `alpha` included by `iota`")
        )
    }
}
