use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The `[build-system]` section of a pyproject.toml as specified in PEP 517
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct BuildSystem {
    /// PEP 508 dependencies required to execute the build system
    pub requires: Vec<String>,
    /// A string naming a Python object that will be used to perform the build
    pub build_backend: Option<String>,
    /// Specify that their backend code is hosted in-tree, this key contains a list of directories
    pub backend_path: Option<Vec<String>>,
}

/// A pyproject.toml as specified in PEP 517
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PyProjectToml {
    /// Build-related data
    pub build_system: BuildSystem,
    /// Project metadata
    pub project: Option<Project>,
}

/// PEP 621 project metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
    /// The name of the project
    pub name: String,
    /// The version of the project as supported by PEP 440
    pub version: Option<String>,
    /// The summary description of the project
    pub description: Option<String>,
    /// The full description of the project (i.e. the README)
    pub readme: Option<ReadMe>,
    /// The Python version requirements of the project
    pub requires_python: Option<String>,
    /// License
    pub license: Option<License>,
    /// The people or organizations considered to be the "authors" of the project
    pub authors: Option<Vec<People>>,
    /// Similar to "authors" in that its exact meaning is open to interpretation
    pub maintainers: Option<Vec<People>>,
    /// The keywords for the project
    pub keywords: Option<Vec<String>>,
    /// Trove classifiers which apply to the project
    pub classifiers: Option<Vec<String>>,
    /// A table of URLs where the key is the URL label and the value is the URL itself
    pub urls: Option<HashMap<String, String>>,
    /// Entry points
    pub entry_points: Option<HashMap<String, HashMap<String, String>>>,
    /// Corresponds to the console_scripts group in the core metadata
    pub scripts: Option<HashMap<String, String>>,
    /// Corresponds to the gui_scripts group in the core metadata
    pub gui_scripts: Option<HashMap<String, String>>,
    /// Project dependencies
    pub dependencies: Option<Vec<String>>,
    /// Optional dependencies
    pub optional_dependencies: Option<HashMap<String, Vec<String>>>,
    /// Specifies which fields listed by PEP 621 were intentionally unspecified
    /// so another tool can/will provide such metadata dynamically.
    pub dynamic: Option<Vec<String>>,
}

/// The full description of the project (i.e. the README).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum ReadMe {
    /// Relative path to a text file containing the full description
    RelativePath(String),
    /// Detailed readme information
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct License {
    /// A relative file path to the file which contains the license for the project
    pub file: Option<String>,
    /// The license content of the project
    pub text: Option<String>,
}

/// Project people contact information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct People {
    /// A valid email name
    pub name: Option<String>,
    /// A valid email address
    pub email: Option<String>,
}

impl PyProjectToml {
    /// Parse `pyproject.toml` content
    pub fn new(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}

#[cfg(test)]
mod tests {
    use super::{PyProjectToml, ReadMe};

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
        let build_system = &project_toml.build_system;
        assert_eq!(build_system.requires, &["maturin"]);
        assert_eq!(build_system.build_backend.as_deref(), Some("maturin"));

        let project = project_toml.project.as_ref().unwrap();
        assert_eq!(project.name, "spam");
        assert_eq!(project.version.as_deref(), Some("2020.0.0"));
        assert_eq!(
            project.description.as_deref(),
            Some("Lovely Spam! Wonderful Spam!")
        );
        assert_eq!(
            project.readme,
            Some(ReadMe::RelativePath("README.rst".to_string()))
        );
        assert_eq!(project.requires_python.as_deref(), Some(">=3.8"));
        assert_eq!(
            project.license.as_ref().unwrap().file.as_deref(),
            Some("LICENSE.txt")
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
}
