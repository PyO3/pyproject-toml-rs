# Changelog

## 0.6.0

 * Update to latest [PEP 639](https://peps.python.org/pep-0639) draft. The `license` key is now an enum that can either be an SPDX identifier or the previous table form, which accepting PEP 639 would deprecate. The previous implementation of a `project.license-expression` key in `pyproject.toml` has been [removed](https://peps.python.org/pep-0639/#define-a-new-top-level-license-expression-key).