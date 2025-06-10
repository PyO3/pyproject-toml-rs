# pyproject-toml-rs

[![Crates.io](https://img.shields.io/crates/v/pyproject-toml.svg)](https://crates.io/crates/pyproject-toml)
[![docs.rs](https://docs.rs/pyproject-toml/badge.svg)](https://docs.rs/pyproject-toml/)

`pyproject.toml` parser in Rust.

## Installation

Add it to your ``Cargo.toml``:

```toml
[dependencies]
pyproject-toml = "0.8"
```

then you are good to go. If you are using Rust 2015 you have to add ``extern crate pyproject_toml`` to your crate root as well.

## Extended parsing

If you want to add additional fields parsing, you can do it with [`serde`](https://github.com/serde-rs/serde)'s
[`flatten`](https://serde.rs/field-attrs.html#flatten) feature and implement the [`Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html) trait,
for example:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PyProjectToml {
    #[serde(flatten)]
    inner: pyproject_toml::PyProjectToml,
    tool: Option<Tool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Tool {
    maturin: Option<ToolMaturin>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolMaturin {
    sdist_include: Option<Vec<String>>,
}

impl std::ops::Deref for PyProjectToml {
    type Target = pyproject_toml::PyProjectToml;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PyProjectToml {
    pub fn new(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }
}
```

## License

This work is released under the MIT license. A copy of the license is provided in the [LICENSE](./LICENSE) file.
