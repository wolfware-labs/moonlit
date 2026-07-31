//! Pure scaffold logic: crate-name validation, value resolution, dep line.

use std::path::Path;

/// Resolved values used to render the scaffold templates.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaffoldValues {
    pub name: String,
    pub namespace: String,
    pub description: String,
    pub license: String,
    /// The full RHS of the `moonlit-sdk = …` dependency line.
    pub sdk_dep: String,
}

/// A crate-safe name: starts with a letter, then letters/digits/`-`/`_`.
pub fn is_valid_crate_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The dependency RHS: pinned crates.io version by default, a path dep when
/// `--sdk-path` is given (for local/in-repo development).
pub fn sdk_dep_line(sdk_path: Option<&Path>) -> String {
    match sdk_path {
        Some(p) => format!("{{ path = \"{}\" }}", p.display()),
        None => "\"0.3.0\"".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_and_invalid_crate_names() {
        assert!(is_valid_crate_name("git"));
        assert!(is_valid_crate_name("my-plugin"));
        assert!(is_valid_crate_name("my_plugin2"));
        assert!(!is_valid_crate_name(""));
        assert!(!is_valid_crate_name("2fast"));
        assert!(!is_valid_crate_name("has space"));
        assert!(!is_valid_crate_name("dots.bad"));
    }

    #[test]
    fn sdk_dep_defaults_to_crates_io() {
        assert_eq!(sdk_dep_line(None), "\"0.3.0\"");
    }

    #[test]
    fn sdk_dep_path_when_given() {
        assert_eq!(
            sdk_dep_line(Some(Path::new("/repo/sdk"))),
            "{ path = \"/repo/sdk\" }"
        );
    }
}
