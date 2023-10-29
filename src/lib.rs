use indexmap::IndexMap;
use pep440_rs::{Version, VersionSpecifiers};
use pep508_rs::Requirement;
use serde::{Deserialize, Serialize};

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
}

/// PEP 621 project metadata
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
    /// The name of the project
    pub name: String,
    /// The version of the project as supported by PEP 440
    pub version: Option<Version>,
    /// The summary description of the project
    pub description: Option<String>,
    /// The full description of the project (i.e. the README)
    pub readme: Option<ReadMe>,
    /// The Python version requirements of the project
    pub requires_python: Option<VersionSpecifiers>,
    /// License
    pub license: Option<License>,
    /// License Files (PEP 639) - https://peps.python.org/pep-0639/#add-license-files-key
    pub license_files: Option<LicenseFiles>,
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

/// License
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum License {
    /// A SPDX license expression, according to PEP 639
    String(String),
    /// A PEP 621 license table. Note that accepting PEP 639 will deprecate this table
    Table {
        /// A relative file path to the file which contains the license for the project
        file: Option<String>,
        /// The license content of the project
        text: Option<String>,
    },
}

/// License-Files
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LicenseFiles {
    /// List of file paths describing `License-File` output
    #[serde(rename = "paths")]
    Paths(Option<Vec<String>>),
    /// List of glob patterns describing `License-File` output
    #[serde(rename = "globs")]
    Globs(Option<Vec<String>>),
}

/// Default value specified by PEP 639
impl Default for LicenseFiles {
    fn default() -> Self {
        LicenseFiles::Globs(Some(vec![
            "LICEN[CS]E*".to_owned(),
            "COPYING*".to_owned(),
            "NOTICE*".to_owned(),
            "AUTHORS*".to_owned(),
        ]))
    }
}

/// Project people contact information
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(expecting = "a table with 'name' and 'email' keys")]
pub struct Contact {
    /// A valid email name
    pub name: Option<String>,
    /// A valid email address
    pub email: Option<String>,
}

impl PyProjectToml {
    /// Parse `pyproject.toml` content
    pub fn new(content: &str) -> Result<Self, toml::de::Error> {
        toml::de::from_str(content)
    }
}

#[cfg(test)]
mod tests {
    use super::{License, LicenseFiles, PyProjectToml, ReadMe};
    use pep440_rs::{Version, VersionSpecifiers};
    use pep508_rs::Requirement;
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
            Some(License::Table {
                file: Some("LICENSE.txt".to_owned()),
                text: None
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
            Some(License::String("MIT OR BSD-3-Clause".to_owned()))
        );
    }

    /// https://peps.python.org/pep-0639/#advanced-example
    #[test]
    fn test_parse_pyproject_toml_license_paths() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
license = "MIT AND (Apache-2.0 OR BSD-2-Clause)"
license-files.paths = [
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
            Some(License::String(
                "MIT AND (Apache-2.0 OR BSD-2-Clause)".to_owned()
            ))
        );
        assert_eq!(
            project.license_files,
            Some(LicenseFiles::Paths(Some(vec![
                "LICENSE".to_owned(),
                "setuptools/_vendor/LICENSE".to_owned(),
                "setuptools/_vendor/LICENSE.APACHE".to_owned(),
                "setuptools/_vendor/LICENSE.BSD".to_owned()
            ])))
        );
    }

    // https://peps.python.org/pep-0639/#advanced-example
    #[test]
    fn test_parse_pyproject_toml_license_globs() {
        let source = r#"[build-system]
requires = ["maturin"]
build-backend = "maturin"

[project]
name = "spam"
license = "MIT AND (Apache-2.0 OR BSD-2-Clause)"
license-files.globs = [
    "LICENSE*",
    "setuptools/_vendor/LICENSE*",
]
"#;
        let project_toml = PyProjectToml::new(source).unwrap();
        let project = project_toml.project.as_ref().unwrap();

        assert_eq!(
            project.license,
            Some(License::String(
                "MIT AND (Apache-2.0 OR BSD-2-Clause)".to_owned()
            ))
        );
        assert_eq!(
            project.license_files,
            Some(LicenseFiles::Globs(Some(vec![
                "LICENSE*".to_owned(),
                "setuptools/_vendor/LICENSE*".to_owned(),
            ])))
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

        assert_eq!(
            project.license_files.clone().unwrap_or_default(),
            LicenseFiles::Globs(Some(vec![
                "LICEN[CS]E*".to_owned(),
                "COPYING*".to_owned(),
                "NOTICE*".to_owned(),
                "AUTHORS*".to_owned(),
            ]))
        );
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
}
