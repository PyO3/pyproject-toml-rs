//! Implementation of PEP 639 cross-language restricted globs.

use glob::{Pattern, PatternError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Pep639GlobError {
    #[error(transparent)]
    PatternError(#[from] PatternError),
    #[error(
        "The parent directory operator (`..`) at position {pos} is not allowed in glob: `{glob}`"
    )]
    ParentDirectory { glob: String, pos: usize },
    #[error("Invalid character `{invalid}` at position {pos} in glob: `{glob}`")]
    InvalidCharacter {
        glob: String,
        pos: usize,
        invalid: char,
    },
    #[error("Only forward slashes are allowed as path separator, invalid character at position {pos} in glob: `{glob}`")]
    InvalidBackslash { glob: String, pos: usize },
    #[error("Invalid character `{invalid}` in range at position {pos} in glob: `{glob}`")]
    InvalidCharacterRange {
        glob: String,
        pos: usize,
        invalid: char,
    },
    #[error("Too many at stars at position {pos} in glob: `{glob}`")]
    TooManyStars { glob: String, pos: usize },
}

/// Parse a PEP 639 `license-files` glob
///
/// The syntax is more restricted than regular globbing in Python or Rust for platform independent
/// results. Since [`glob::Pattern`] is a superset over this format, we can use it after validating
/// that no unsupported features are in the string.
///
/// From [PEP 639](https://peps.python.org/pep-0639/#add-license-files-key):
///
/// > Its value is an array of strings which MUST contain valid glob patterns,
/// > as specified below:
/// >
/// > - Alphanumeric characters, underscores (`_`), hyphens (`-`) and dots (`.`)
/// >   MUST be matched verbatim.
/// >
/// > - Special glob characters: `*`, `?`, `**` and character ranges: `[]`
/// >   containing only the verbatim matched characters MUST be supported.
/// >   Within `[...]`, the hyphen indicates a range (e.g. `a-z`).
/// >   Hyphens at the start or end are matched literally.
/// >
/// > - Path delimiters MUST be the forward slash character (`/`).
/// >   Patterns are relative to the directory containing `pyproject.toml`,
/// >   therefore the leading slash character MUST NOT be used.
/// >
/// > - Parent directory indicators (`..`) MUST NOT be used.
/// >
/// > Any characters or character sequences not covered by this specification are
/// > invalid. Projects MUST NOT use such values.
/// > Tools consuming this field MAY reject invalid values with an error.
pub fn parse_pep639_glob(glob: &str) -> Result<Pattern, Pep639GlobError> {
    check_pep639_glob(glob)?;
    Ok(Pattern::new(glob)?)
}

/// Check if a glob pattern is valid according to PEP 639 rules.
///
/// See [parse_pep639_glob].
pub fn check_pep639_glob(glob: &str) -> Result<(), Pep639GlobError> {
    let mut chars = glob.chars().enumerate().peekable();
    // A `..` is on a parent directory indicator at the start of the string or after a directory
    // separator.
    let mut start_or_slash = true;
    while let Some((pos, c)) = chars.next() {
        // `***` or `**literals` can be correctly represented with less stars. They are banned by
        // `glob`, they are allowed by `globset` and PEP 639 is ambiguous, so we're filtering them
        // out.
        if c == '*' {
            let mut star_run = 1;
            while let Some((_, c)) = chars.peek() {
                if *c == '*' {
                    star_run += 1;
                    chars.next();
                } else {
                    break;
                }
            }
            if star_run >= 3 {
                return Err(Pep639GlobError::TooManyStars {
                    glob: glob.to_string(),
                    // We don't update pos for the stars.
                    pos,
                });
            } else if star_run == 2 {
                if let Some((_, c)) = chars.peek() {
                    if *c != '/' {
                        return Err(Pep639GlobError::TooManyStars {
                            glob: glob.to_string(),
                            // We don't update pos for the stars.
                            pos,
                        });
                    }
                }
            }
            start_or_slash = false;
        } else if c.is_alphanumeric() || matches!(c, '_' | '-' | '?') {
            start_or_slash = false;
        } else if c == '.' {
            if start_or_slash && matches!(chars.peek(), Some((_, '.'))) {
                return Err(Pep639GlobError::ParentDirectory {
                    pos,
                    glob: glob.to_string(),
                });
            }
            start_or_slash = false;
        } else if c == '/' {
            start_or_slash = true;
        } else if c == '[' {
            for (pos, c) in chars.by_ref() {
                if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') {
                    // Allowed.
                } else if c == ']' {
                    break;
                } else {
                    return Err(Pep639GlobError::InvalidCharacterRange {
                        glob: glob.to_string(),
                        pos,
                        invalid: c,
                    });
                }
            }
            start_or_slash = false;
        } else if c == '\\' {
            return Err(Pep639GlobError::InvalidBackslash {
                glob: glob.to_string(),
                pos,
            });
        } else {
            return Err(Pep639GlobError::InvalidCharacter {
                glob: glob.to_string(),
                pos,
                invalid: c,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn test_error() {
        let parse_err = |glob| parse_pep639_glob(glob).unwrap_err().to_string();
        assert_snapshot!(
            parse_err(".."),
            @"The parent directory operator (`..`) at position 0 is not allowed in glob: `..`"
        );
        assert_snapshot!(
            parse_err("licenses/.."),
            @"The parent directory operator (`..`) at position 9 is not allowed in glob: `licenses/..`"
        );
        assert_snapshot!(
            parse_err("licenses/LICEN!E.txt"),
            @"Invalid character `!` at position 14 in glob: `licenses/LICEN!E.txt`"
        );
        assert_snapshot!(
            parse_err("licenses/LICEN[!C]E.txt"),
            @"Invalid character `!` in range at position 15 in glob: `licenses/LICEN[!C]E.txt`"
        );
        assert_snapshot!(
            parse_err("licenses/LICEN[C?]E.txt"),
            @"Invalid character `?` in range at position 16 in glob: `licenses/LICEN[C?]E.txt`"
        );
        assert_snapshot!(
            parse_err("******"),
            @"Too many at stars at position 0 in glob: `******`"
        );
        assert_snapshot!(
            parse_err("licenses/**license"),
            @"Too many at stars at position 9 in glob: `licenses/**license`"
        );
        assert_snapshot!(
            parse_err("licenses/***/licenses.csv"),
            @"Too many at stars at position 9 in glob: `licenses/***/licenses.csv`"
        );
        assert_snapshot!(
            parse_err(r"licenses\eula.txt"),
            @r"Only forward slashes are allowed as path separator, invalid character at position 8 in glob: `licenses\eula.txt`"
        );
        assert_snapshot!(
            parse_err(r"**/@test"),
            @"Invalid character `@` at position 3 in glob: `**/@test`"
        );
        // Backslashes are not allowed
        assert_snapshot!(
            parse_err(r"public domain/Gulliver\\'s Travels.txt"),
            @r"Invalid character ` ` at position 6 in glob: `public domain/Gulliver\\'s Travels.txt`"
        );
    }

    #[test]
    fn test_valid() {
        let cases = [
            "licenses/*.txt",
            "licenses/**/*.txt",
            "LICEN[CS]E.txt",
            "LICEN?E.txt",
            "[a-z].txt",
            "[a-z._-].txt",
            "*/**",
            "LICENSE..txt",
            "LICENSE_file-1.txt",
            // (google translate)
            "licenses/라이센스*.txt",
            "licenses/ライセンス*.txt",
            "licenses/执照*.txt",
            "src/**",
        ];
        for case in cases {
            parse_pep639_glob(case).unwrap();
        }
    }
}
