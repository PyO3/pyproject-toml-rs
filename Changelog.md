# Changelog

## Unreleased

## 0.13.7

* Normalize extra names in optional dependencies

## 0.13.6

* Support resolving optional dependencies and dependency groups
* Update toml to 0.9

## 0.13.5

* Better PEP 639 `license-files` glob validation and error messages from uv
* A public `check_pep639_glob` function for using the PEP 639 support with a different glob crate.

## 0.13.4

* Update pep440_rs to 0.7.2
* Update pep508_rs to 0.9.1
* Ensure wasm32-unknown-unknown support

## 0.13.3

* Make `license` and `license_files` public again
* Add accessors for email / name on Contact
* Add a method to resolve dependency groups into concrete lists of dependencies

## 0.13.2

* Make `Contact` definition strict

## 0.13.1

* Fix `Contact` definition

## 0.13.0

* Update to the provisional PEP 639. This is technically a breaking change, but only for fields previously in draft
* Update pep440_rs to 0.7.1
* Update pep508_rs to 0.8.0

## 0.12.0

* Support dependency groups (PEP 735)

## 0.11.0

* Update pep440_rs to 0.6.0
* Update pep508_rs to 0.6.0

## 0.8.0

* The `build_system` table is now optional. There are many projects that use pyproject.toml for tool configuration
  without specifying a build backend, which this change reflects.

## 0.6.0

* Update to latest [PEP 639](https://peps.python.org/pep-0639) draft. The `license` key is now an enum that can either
  be an SPDX identifier or the previous table form, which accepting PEP 639 would deprecate. The previous implementation
  of a `project.license-expression` key in `pyproject.toml` has
  been [removed](https://peps.python.org/pep-0639/#define-a-new-top-level-license-expression-key).
