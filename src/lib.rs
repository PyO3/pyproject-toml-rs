#[cfg(feature = "pep639-glob")]
mod pep639_glob;
mod resolution;

#[cfg(feature = "pep639-glob")]
pub use pep639_glob::{check_pep639_glob, parse_pep639_glob, Pep639GlobError};
pub use resolution::ResolveError;

use indexmap::IndexMap;
use pep440_rs::{Version, VersionSpecifiers};
use pep508_rs::Requirement;
use resolution::resolve;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::path::PathBuf;

/// The `[build-system]` section of a pyproject.toml as specified in PEP 517
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BuildSystem {
    /// PEP 508 dependencies required to execute the build system
    pub requires: Vec<Requirement>,
    /// A string naming a Python object that will be used to perform the build
    pub build_backend: Option<String>,
    /// Specify that their backend code is hosted in-tree, this key contains a list of directories
    pub backend_path: Option<Vec<String>>,
}

/// A pyproject.toml as specified in PEP 517
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PyProjectToml {
    /// Build-related data
    pub build_system: Option<BuildSystem>,
    /// Project metadata
    pub project: Option<Project>,
    /// Dependency groups table
    pub dependency_groups: Option<DependencyGroups>,
}

/// PEP 621 project metadata
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
    /// The name of the project
    // TODO: This should become a `PackageName` in the next breaking release.
    pub name: String,
    /// The version of the project as supported by PEP 440
    pub version: Option<Version>,
    /// The summary description of the project
    pub description: Option<String>,
    /// The full description of the project (i.e. the README)
    pub readme: Option<ReadMe>,
    /// The Python version requirements of the project
    pub requires_python: Option<VersionSpecifiers>,
    /// The license under which the project is distributed
    ///
    /// Supports both the current standard and the provisional PEP 639
    pub license: Option<License>,
    /// The paths to files containing licenses and other legal notices to be distributed with the
    /// project.
    ///
    /// Use `parse_pep639_glob` from the optional `pep639-glob` feature to find the matching files.
    ///
    /// Note that this doesn't check the PEP 639 rules for combining `license_files` and `license`.
    ///
    /// From the provisional PEP 639
    pub license_files: Option<Vec<String>>,
    /// The people or organizations considered to be the "authors" of the project
    pub authors: Option<Vec<Contact>>,
    /// Similar to "authors" in that its exact meaning is open to interpretation
    pub maintainers: Option<Vec<Contact>>,
    /// The keywords for the project
    pub keywords: Option<Vec<String>>,
    /// Trove classifiers which apply to the project
    pub classifiers: Option<Vec<String>>,
    /// A table of URLs where the key is the URL label and the value is the URL itself
    pub urls: Option<IndexMap<String, String>>,
    /// Entry points
    pub entry_points: Option<IndexMap<String, IndexMap<String, String>>>,
    /// Corresponds to the console_scripts group in the core metadata
    pub scripts: Option<IndexMap<String, String>>,
    /// Corresponds to the gui_scripts group in the core metadata
    pub gui_scripts: Option<IndexMap<String, String>>,
    /// Project dependencies
    pub dependencies: Option<Vec<Requirement>>,
    /// Optional dependencies
    // TODO: The `String` should become a `ExtraName` in the next breaking release.
    pub optional_dependencies: Option<IndexMap<String, Vec<Requirement>>>,
    /// Specifies which fields listed by PEP 621 were intentionally unspecified
    /// so another tool can/will provide such metadata dynamically.
    pub dynamic: Option<Vec<String>>,
}

impl Project {
    /// Initializes the only field mandatory in PEP 621 (`name`) and leaves everything else empty
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            description: None,
            readme: None,
            requires_python: None,
            license: None,
            license_files: None,
            authors: None,
            maintainers: None,
            keywords: None,
            classifiers: None,
            urls: None,
            entry_points: None,
            scripts: None,
            gui_scripts: None,
            dependencies: None,
            optional_dependencies: None,
            dynamic: None,
        }
    }
}

/// The full description of the project (i.e. the README).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum ReadMe {
    /// Relative path to a text file containing the full description
    RelativePath(String),
    /// Detailed readme information
    #[serde(rename_all = "kebab-case")]
    Table {
        /// A relative path to a file containing the full description
        file: Option<String>,
        /// Full description
        text: Option<String>,
        /// The content-type of the full description
        content_type: Option<String>,
    },
}

/// The optional `project.license` key
///
/// Specified in <https://packaging.python.org/en/latest/specifications/pyproject-toml/#license>.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum License {
    /// An SPDX Expression.
    ///
    /// Note that this doesn't check the validity of the SPDX expression or PEP 639 rules.
    ///
    /// From the provisional PEP 639.
    Spdx(String),
    Text {
        /// The full text of the license.
        text: String,
    },
    File {
        /// The file containing the license text.
        file: PathBuf,
    },
}

/// A `project.authors` or `project.maintainers` entry.
///
/// Specified in
/// <https://packaging.python.org/en/latest/specifications/pyproject-toml/#authors-maintainers>.
///
/// The entry is derived from the email format of `John Doe <john.doe@example.net>`. You need to
/// provide at least name or email.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
// deny_unknown_fields prevents using the name field when the email is not a string.
#[serde(
    untagged,
    deny_unknown_fields,
    expecting = "a table with 'name' and/or 'email' keys"
)]
pub enum Contact {
    /// TODO(konsti): RFC 822 validation.
    NameEmail { name: String, email: String },
    /// TODO(konsti): RFC 822 validation.
    Name { name: String },
    /// TODO(konsti): RFC 822 validation.
    Email { email: String },
}

impl Contact {
    /// Returns the name of the contact.
    pub fn name(&self) -> Option<&str> {
        match self {
            Contact::NameEmail { name, .. } | Contact::Name { name } => Some(name),
            Contact::Email { .. } => None,
        }
    }

    /// Returns the email of the contact.
    pub fn email(&self) -> Option<&str> {
        match self {
            Contact::NameEmail { email, .. } | Contact::Email { email } => Some(email),
            Contact::Name { .. } => None,
        }
    }
}

/// The `[dependency-groups]` section of pyproject.toml, as specified in PEP 735
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
// TODO: The `String` should become a `ExtraName` in the next breaking release.
pub struct DependencyGroups(pub IndexMap<String, Vec<DependencyGroupSpecifier>>);

impl Deref for DependencyGroups {
    type Target = IndexMap<String, Vec<DependencyGroupSpecifier>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A specifier item in a Dependency Group
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", untagged)]
#[allow(clippy::large_enum_variant)]
pub enum DependencyGroupSpecifier {
    /// PEP 508 requirement string
    String(Requirement),
    /// Include another dependency group
    #[serde(rename_all = "kebab-case")]
    Table {
        /// The name of the group to include
        include_group: String,
    },
}

/// Optional dependencies and dependency groups resolved into flat lists of requirements that are
/// not self-referential
///
/// Note that `project.name` is required to resolve self-referential optional dependencies
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedDependencies {
    pub optional_dependencies: IndexMap<String, Vec<Requirement>>,
    pub dependency_groups: IndexMap<String, Vec<Requirement>>,
}

impl PyProjectToml {
    /// Parse `pyproject.toml` content
    pub fn new(content: &str) -> Result<Self, toml::de::Error> {
        toml::de::from_str(content)
    }

    /// Resolve the optional dependencies (extras) and dependency groups into flat lists of
    /// requirements.
    ///
    /// This function will recursively resolve all optional dependency groups and dependency groups,
    /// including those that reference other groups. It will return an error if
    ///  - there is a cycle in the groups, or
    ///  - a group references another group that does not exist.
    ///
    /// Resolving self-referential optional dependencies requires `project.name` to be set.
    ///
    /// Note: This method makes no guarantee about the order of items and whether duplicates are
    /// removed or not.
    pub fn resolve(&self) -> Result<ResolvedDependencies, ResolveError> {
        let self_reference_name = self.project.as_ref().map(|p| p.name.as_str());
        let optional_dependencies = self
            .project
            .as_ref()
            .and_then(|p| p.optional_dependencies.as_ref());
        let dependency_groups = self.dependency_groups.as_ref();

        let resolved_dependencies = resolve(
            self_reference_name,
            optional_dependencies,
            dependency_groups,
        )?;

        Ok(resolved_dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::{DependencyGroupSpecifier, License, PyProjectToml, ReadMe};
    use pep440_rs::{Version, VersionSpecifiers};
    use pep508_rs::Requirement;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn test_parse_pyproject_toml() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
version = "2020.0.0"
description = "Lovely Spam! Wonderful Spam!"
readme = "README.rst"
requires-python = ">=3.8"
license = {file = "LICENSE.txt"}
keywords = ["egg", "bacon", "sausage", "tomatoes", "Lobster Thermidor"]
authors = [
  {email = "hi@pradyunsg.me"},
  {name = "Tzu-Ping Chung"}
]
maintainers = [
  {name = "Brett Cannon", email = "brett@python.org"}
]
classifiers = [
  "Development Status :: 4 - Beta",
  "Programming Language :: Python"
]

dependencies = [
  "httpx",
  "gidgethub[httpx]>4.0.0",
  "django>2.1; os_name != 'nt'",
  "django>2.0; os_name == 'nt'"
]

[project.optional-dependencies]
test = [
  "pytest < 5.0.0",
  "pytest-cov[all]"
]

[project.urls]
homepage = "example.com"
documentation = "readthedocs.org"
repository = "github.com"
changelog = "github.com/me/spam/blob/master/CHANGELOG.md"

[project.scripts]
spam-cli = "spam:main_cli"

[project.gui-scripts]
spam-gui = "spam:main_gui"

[project.entry-points."spam.magical"]
tomatoes = "spam:main_tomatoes""#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let build_system = &project_toml.build_system.unwrap();
        assert_eq!(
            build_system.requires,
            &[Requirement::from_str("maturin").unwrap()]
        );
        assert_eq!(build_system.build_backend.as_deref(), Some("maturin"));

        let project = project_toml.project.as_ref().unwrap();
        assert_eq!(project.name, "spam");
        assert_eq!(
            project.version,
            Some(Version::from_str("2020.0.0").unwrap())
        );
        assert_eq!(
            project.description.as_deref(),
            Some("Lovely Spam! Wonderful Spam!")
        );
        assert_eq!(
            project.readme,
            Some(ReadMe::RelativePath("README.rst".to_string()))
        );
        assert_eq!(
            project.requires_python,
            Some(VersionSpecifiers::from_str(">=3.8").unwrap())
        );
        assert_eq!(
            project.license,
            Some(License::File {
                file: PathBuf::from("LICENSE.txt"),
            })
        );
        assert_eq!(
            project.keywords.as_ref().unwrap(),
            &["egg", "bacon", "sausage", "tomatoes", "Lobster Thermidor"]
        );
        assert_eq!(
            project.scripts.as_ref().unwrap()["spam-cli"],
            "spam:main_cli"
        );
        assert_eq!(
            project.gui_scripts.as_ref().unwrap()["spam-gui"],
            "spam:main_gui"
        );
    }

    #[test]
    fn test_parse_pyproject_toml_license_expression() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
license = "MIT OR BSD-3-Clause"
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let project = project_toml.project.as_ref().unwrap();
        assert_eq!(
            project.license,
            Some(License::Spdx("MIT OR BSD-3-Clause".to_owned()))
        );
    }

    /// https://peps.python.org/pep-0639/
    #[test]
    fn test_parse_pyproject_toml_license_paths() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
license = "MIT AND (Apache-2.0 OR BSD-2-Clause)"
license-files = [
    "LICENSE",
    "setuptools/_vendor/LICENSE",
    "setuptools/_vendor/LICENSE.APACHE",
    "setuptools/_vendor/LICENSE.BSD",
]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let project = project_toml.project.as_ref().unwrap();

        assert_eq!(
            project.license,
            Some(License::Spdx(
                "MIT AND (Apache-2.0 OR BSD-2-Clause)".to_owned()
            ))
        );
        assert_eq!(
            project.license_files,
            Some(vec![
                "LICENSE".to_owned(),
                "setuptools/_vendor/LICENSE".to_owned(),
                "setuptools/_vendor/LICENSE.APACHE".to_owned(),
                "setuptools/_vendor/LICENSE.BSD".to_owned()
            ])
        );
    }

    // https://peps.python.org/pep-0639/
    #[test]
    fn test_parse_pyproject_toml_license_globs() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
license = "MIT AND (Apache-2.0 OR BSD-2-Clause)"
license-files = [
    "LICENSE*",
    "setuptools/_vendor/LICENSE*",
]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let project = project_toml.project.as_ref().unwrap();

        assert_eq!(
            project.license,
            Some(License::Spdx(
                "MIT AND (Apache-2.0 OR BSD-2-Clause)".to_owned()
            ))
        );
        assert_eq!(
            project.license_files,
            Some(vec![
                "LICENSE*".to_owned(),
                "setuptools/_vendor/LICENSE*".to_owned(),
            ])
        );
    }

    #[test]
    fn test_parse_pyproject_toml_default_license_files() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let project = project_toml.project.as_ref().unwrap();

        // Changed from the PEP 639 draft.
        assert_eq!(project.license_files.clone(), None);
    }

    #[test]
    fn test_parse_pyproject_toml_readme_content_type() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
readme = {text = "ReadMe!", content-type = "text/plain"}
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let project = project_toml.project.as_ref().unwrap();

        assert_eq!(
            project.readme,
            Some(ReadMe::Table {
                file: None,
                text: Some("ReadMe!".to_string()),
                content_type: Some("text/plain".to_string())
            })
        );
    }

    #[test]
    fn test_parse_pyproject_toml_dependency_groups() {
        let source = r#"[dependency-groups]
alpha = ["beta", "gamma", "delta"]
epsilon = ["eta<2.0", "theta==2024.09.01"]
iota = [{include-group = "alpha"}]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let dependency_groups = project_toml.dependency_groups.as_ref().unwrap();

        assert_eq!(
            dependency_groups["alpha"],
            vec![
                DependencyGroupSpecifier::String(Requirement::from_str("beta").unwrap()),
                DependencyGroupSpecifier::String(Requirement::from_str("gamma").unwrap()),
                DependencyGroupSpecifier::String(Requirement::from_str("delta").unwrap(),)
            ]
        );
        assert_eq!(
            dependency_groups["epsilon"],
            vec![
                DependencyGroupSpecifier::String(Requirement::from_str("eta<2.0").unwrap()),
                DependencyGroupSpecifier::String(
                    Requirement::from_str("theta==2024.09.01").unwrap()
                )
            ]
        );
        assert_eq!(
            dependency_groups["iota"],
            vec![DependencyGroupSpecifier::Table {
                include_group: "alpha".to_string()
            }]
        );
    }

    #[test]
    fn invalid_email() {
        let source = r#"
[project]
name = "hello-world"
version = "0.1.0"
# Ensure that the spans from toml handle utf-8 correctly
authors = [
    { name = "Z͑ͫ̓ͪ̂ͫ̽͏̴̙̤̞͉͚̯̞̠͍A̴̵̜̰͔ͫ͗͢L̠ͨͧͩ͘G̴̻͈͍͔̹̑͗̎̅͛́Ǫ̵̹̻̝̳͂̌̌͘", email = 1 }
]
"#;
        let err = PyProjectToml::new(source).unwrap_err();
        assert_eq!(
            err.to_string(),
            "TOML parse error at line 6, column 11
  |
6 | authors = [
  |           ^
a table with 'name' and/or 'email' keys
"
        );
    }

    #[test]
    fn test_contact_accessors() {
        let contact = super::Contact::NameEmail {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };

        assert_eq!(contact.name(), Some("John Doe"));
        assert_eq!(contact.email(), Some("john@example.com"));

        let contact = super::Contact::Name {
            name: "John Doe".to_string(),
        };

        assert_eq!(contact.name(), Some("John Doe"));
        assert_eq!(contact.email(), None);

        let contact = super::Contact::Email {
            email: "john@example.com".to_string(),
        };

        assert_eq!(contact.name(), None);
        assert_eq!(contact.email(), Some("john@example.com"));
    }
}
