use crate::{DependencyGroupSpecifier, DependencyGroups, ResolvedDependencies};
use indexmap::IndexMap;
use pep508_rs::{ExtraName, Requirement};
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

/// Normalize a group/extra name according to PEP 685.
fn normalize_name(name: &str) -> String {
    ExtraName::from_str(name)
        .map(|extra| extra.to_string())
        .unwrap_or_else(|_| name.to_string())
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct ResolveError(#[from] ResolveErrorKind);

#[derive(Debug, Error)]
pub enum ResolveErrorKind {
    #[error("Failed to find optional dependency `{name}` included by {included_by}")]
    OptionalDependencyNotFound { name: String, included_by: Item },
    #[error("Failed to find dependency group `{name}` included by {included_by}")]
    DependencyGroupNotFound { name: String, included_by: Item },
    #[error("Cycles are not supported: {0}")]
    DependencyGroupCycle(Cycle),
}

/// A cycle in the recursion.
#[derive(Debug)]
pub struct Cycle(Vec<Item>);

/// Display a cycle, e.g., `a -> b -> c -> a`.
impl Display for Cycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Some((first, rest)) = self.0.split_first() else {
            return Ok(());
        };
        write!(f, "{first}")?;
        for group in rest {
            write!(f, " -> {group}")?;
        }
        write!(f, " -> {first}")?;
        Ok(())
    }
}

/// A reference to either an optional dependency or a dependency group.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Item {
    Extra(String),
    Group(String),
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Item::Extra(extra) => write!(f, "extra:{extra}",),
            Item::Group(group) => {
                write!(f, "group:{group}")
            }
        }
    }
}

pub(crate) fn resolve(
    self_reference_name: Option<&str>,
    optional_dependencies: Option<&IndexMap<String, Vec<Requirement>>>,
    dependency_groups: Option<&DependencyGroups>,
) -> Result<ResolvedDependencies, ResolveError> {
    let mut resolved_dependencies = ResolvedDependencies::default();

    // Resolve optional dependencies, which may only reference optional dependencies.
    if let Some(optional_dependencies) = optional_dependencies {
        for extra in optional_dependencies.keys() {
            resolve_optional_dependency(
                extra,
                optional_dependencies,
                &mut resolved_dependencies,
                &mut Vec::new(),
                self_reference_name,
            )?;
        }
    }

    // Resolve dependency groups, which may reference dependency groups and optional dependencies.
    if let Some(dependency_groups) = dependency_groups {
        for group in dependency_groups.keys() {
            // It's a reference to other groups. Recurse into them
            resolve_dependency_group(
                group,
                optional_dependencies.unwrap_or(&IndexMap::default()),
                dependency_groups,
                &mut resolved_dependencies,
                &mut Vec::new(),
                self_reference_name,
            )?;
        }
    }

    Ok(resolved_dependencies)
}

/// Resolves a single optional dependency.
fn resolve_optional_dependency(
    extra: &str,
    optional_dependencies: &IndexMap<String, Vec<Requirement>>,
    resolved: &mut ResolvedDependencies,
    parents: &mut Vec<Item>,
    project_name: Option<&str>,
) -> Result<Vec<Requirement>, ResolveError> {
    if let Some(requirements) = resolved.optional_dependencies.get(extra) {
        return Ok(requirements.clone());
    }

    let normalized_extra = normalize_name(extra);

    // Find the key in optional_dependencies by comparing normalized versions
    // TODO: next breaking release remove this once Extra is added
    let unresolved_requirements = optional_dependencies
        .iter()
        .find(|(key, _)| normalize_name(key) == normalized_extra)
        .map(|(_, reqs)| reqs);

    let Some(unresolved_requirements) = unresolved_requirements else {
        let parent = parents
            .iter()
            .last()
            .expect("missing optional dependency must have parent")
            .clone();
        return Err(ResolveErrorKind::OptionalDependencyNotFound {
            name: extra.to_string(),
            included_by: parent,
        }
        .into());
    };

    // Check for cycles
    let item = Item::Extra(extra.to_string());
    if parents.contains(&item) {
        return Err(ResolveErrorKind::DependencyGroupCycle(Cycle(parents.clone())).into());
    }
    parents.push(item);

    // Recurse into references, and add their resolved requirements to our own requirements.
    let mut resolved_requirements = Vec::with_capacity(unresolved_requirements.len());
    for unresolved_requirement in unresolved_requirements.iter() {
        // TODO: This should become a `PackageName` in the next breaking release.
        if project_name
            .is_some_and(|project_name| project_name == unresolved_requirement.name.to_string())
        {
            // Resolve each extra individually, as each refers to a different optional
            // dependency entry.
            for extra in &unresolved_requirement.extras {
                let extra_string = extra.to_string();
                resolved_requirements.extend(resolve_optional_dependency(
                    &extra_string,
                    optional_dependencies,
                    resolved,
                    parents,
                    project_name,
                )?);
            }
        } else {
            resolved_requirements.push(unresolved_requirement.clone())
        }
    }
    resolved
        .optional_dependencies
        .insert(extra.to_string(), resolved_requirements.clone());
    parents.pop();
    Ok(resolved_requirements)
}

/// Resolves a single dependency group.
fn resolve_dependency_group(
    dep_group: &String,
    optional_dependencies: &IndexMap<String, Vec<Requirement>>,
    dependency_groups: &DependencyGroups,
    resolved: &mut ResolvedDependencies,
    parents: &mut Vec<Item>,
    project_name: Option<&str>,
) -> Result<Vec<Requirement>, ResolveError> {
    if let Some(requirements) = resolved.dependency_groups.get(dep_group) {
        return Ok(requirements.clone());
    }

    let Some(unresolved_requirements) = dependency_groups.get(dep_group) else {
        let parent = parents
            .iter()
            .last()
            .expect("missing optional dependency must have parent")
            .clone();
        return Err(ResolveErrorKind::DependencyGroupNotFound {
            name: dep_group.to_string(),
            included_by: parent,
        }
        .into());
    };

    // Check for cycles
    let item = Item::Group(dep_group.to_string());
    if parents.contains(&item) {
        return Err(ResolveErrorKind::DependencyGroupCycle(Cycle(parents.clone())).into());
    }
    parents.push(item);

    // Otherwise, perform recursion, as required, on the dependency group's specifiers
    let mut resolved_requirements = Vec::with_capacity(unresolved_requirements.len());
    for unresolved_requirement in unresolved_requirements.iter() {
        match unresolved_requirement {
            DependencyGroupSpecifier::String(spec) => {
                if project_name.is_some_and(|project_name| project_name == spec.name.to_string()) {
                    for extra in &spec.extras {
                        resolved_requirements.extend(resolve_optional_dependency(
                            extra.as_ref(),
                            optional_dependencies,
                            resolved,
                            parents,
                            project_name,
                        )?);
                    }
                } else {
                    resolved_requirements.push(spec.clone())
                }
            }
            DependencyGroupSpecifier::Table { include_group } => {
                resolved_requirements.extend(resolve_dependency_group(
                    include_group,
                    optional_dependencies,
                    dependency_groups,
                    resolved,
                    parents,
                    project_name,
                )?);
            }
        }
    }
    // Add the resolved group to IndexMap
    resolved
        .dependency_groups
        .insert(dep_group.to_string(), resolved_requirements.clone());
    parents.pop();
    Ok(resolved_requirements)
}

#[cfg(test)]
mod tests {
    use pep508_rs::Requirement;
    use std::str::FromStr;

    use crate::resolution::{resolve_optional_dependency, Item};
    use crate::{PyProjectToml, ResolvedDependencies};

    #[test]
    fn parse_pyproject_toml_optional_dependencies_resolve() {
        let source = r#"[project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = ["spam[alpha]"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();

        assert_eq!(
            resolved_dependencies.optional_dependencies["iota"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }

    #[test]
    fn parse_pyproject_toml_optional_dependencies_cycle() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["spam[iota]"]
            iota = ["spam[alpha]"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        assert_eq!(
            pyproject_toml.resolve().unwrap_err().to_string(),
            "Cycles are not supported: extra:alpha -> extra:iota -> extra:alpha"
        )
    }

    #[test]
    fn parse_pyproject_toml_optional_dependencies_missing_include() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            iota = ["spam[alpha]"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        assert_eq!(
            pyproject_toml.resolve().unwrap_err().to_string(),
            "Failed to find optional dependency `alpha` included by extra:iota"
        )
    }

    #[test]
    fn parse_pyproject_toml_optional_dependencies_missing_top_level() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            alpha = ["beta"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let mut resolved = ResolvedDependencies::default();
        let err = resolve_optional_dependency(
            "foo",
            pyproject_toml
                .project
                .as_ref()
                .unwrap()
                .optional_dependencies
                .as_ref()
                .unwrap(),
            &mut resolved,
            &mut vec![Item::Extra("bar".to_string())],
            Some("spam"),
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Failed to find optional dependency `foo` included by extra:bar"
        );
    }

    #[test]
    fn parse_pyproject_toml_dependency_groups_resolve() {
        let source = r#"
            [dependency-groups]
            alpha = ["beta", "gamma", "delta"]
            epsilon = ["eta<2.0", "theta==2024.09.01"]
            iota = [{include-group = "alpha"}]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();

        assert_eq!(
            resolved_dependencies.dependency_groups["iota"],
            vec![
                Requirement::from_str("beta").unwrap(),
                Requirement::from_str("gamma").unwrap(),
                Requirement::from_str("delta").unwrap()
            ]
        );
    }

    #[test]
    fn parse_pyproject_toml_dependency_groups_cycle() {
        let source = r#"
            [dependency-groups]
            alpha = [{include-group = "iota"}]
            iota = [{include-group = "alpha"}]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        assert_eq!(
            pyproject_toml.resolve().unwrap_err().to_string(),
            "Cycles are not supported: group:alpha -> group:iota -> group:alpha"
        )
    }

    #[test]
    fn parse_pyproject_toml_dependency_groups_missing_include() {
        let source = r#"
            [dependency-groups]
            iota = [{include-group = "alpha"}]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        assert_eq!(
            pyproject_toml.resolve().unwrap_err().to_string(),
            "Failed to find dependency group `alpha` included by group:iota"
        )
    }

    #[test]
    fn parse_pyproject_toml_dependency_groups_with_optional_dependencies() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            test = ["pytest"]

            [dependency-groups]
            dev = ["spam[test]"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();
        assert_eq!(
            resolved_dependencies.dependency_groups["dev"],
            vec![Requirement::from_str("pytest").unwrap()]
        );
    }

    #[test]
    fn name_collision() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            dev = ["pytest"]

            [dependency-groups]
            dev = ["ruff"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();
        assert_eq!(
            resolved_dependencies.optional_dependencies["dev"],
            vec![Requirement::from_str("pytest").unwrap()]
        );
        assert_eq!(
            resolved_dependencies.dependency_groups["dev"],
            vec![Requirement::from_str("ruff").unwrap()]
        );
    }

    #[test]
    fn optional_dependencies_are_not_dependency_groups() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            test = ["pytest"]

            [dependency-groups]
            dev = ["spam[test]"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();
        assert!(resolved_dependencies
            .optional_dependencies
            .contains_key("test"));
        assert!(!resolved_dependencies.dependency_groups.contains_key("test"));
        assert!(resolved_dependencies.dependency_groups.contains_key("dev"));
    }

    #[test]
    fn mixed_resolution() {
        let source = r#"
            [project]
            name = "spam"

            [project.optional-dependencies]
            test = ["pytest"]
            numpy = ["numpy"]

            [dependency-groups]
            dev = ["spam[test]"]
            test = ["spam[numpy]"]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();
        assert_eq!(
            resolved_dependencies.dependency_groups["dev"],
            vec![Requirement::from_str("pytest").unwrap()]
        );
        assert_eq!(
            resolved_dependencies.dependency_groups["test"],
            vec![Requirement::from_str("numpy").unwrap()]
        );
    }

    #[test]
    fn optional_dependencies_with_underscores() {
        // Test that optional dependency group names with underscores are normalized
        // when referenced in extras. PEP 685 specifies that extras should be normalized
        // by replacing _, ., - with a single -.
        let source = r#"
            [project]
            name = "foo"

            [project.optional-dependencies]
            all = [
              "foo[group-one]",
              "foo[group_two]",
            ]
            group_one = [
              "anyio>=4.9.0",
            ]
            group-two = [
              "trio>=0.31.0",
            ]
        "#;
        let pyproject_toml = PyProjectToml::new(source).unwrap();
        let resolved_dependencies = pyproject_toml.resolve().unwrap();

        // Both group-one and group_two should resolve correctly
        assert_eq!(
            resolved_dependencies.optional_dependencies["all"],
            vec![
                Requirement::from_str("anyio>=4.9.0").unwrap(),
                Requirement::from_str("trio>=0.31.0").unwrap(),
            ]
        );
    }
}
