//! Resolve the working directory and pipeline file from CLI args, and best-effort peek the
//! pipeline name/stages for the startup header. User-facing failures are `EngineError::Config`
//! (exit 2), matching the engine's config diagnostics.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A CLI-side input error (missing/invalid file or working dir). Exit code 2 (configuration),
/// rendered via miette like the engine's config diagnostics but without a source span. This is a
/// CLI-local type — the engine's `ConfigDiagnostic` has no source-less constructor, and these
/// errors precede having any YAML to point at.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{0}")]
#[diagnostic(code(moonlit::cli::input))]
pub struct InputError(pub String);

impl InputError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
    pub fn exit_code(&self) -> i32 {
        2
    }
}

/// The resolved inputs handed to the engine and header.
pub struct ResolvedInput {
    pub working_directory: PathBuf,
    /// File name actually used (e.g. `release.yml`), for the header line.
    pub chosen_name: String,
    /// Pipeline YAML content.
    pub yaml: String,
}

/// Candidate file names tried, in order, when `-f` is not given.
const DEFAULT_NAMES: [&str; 2] = ["release.yml", "release.yaml"];

/// Resolve CLI inputs from file and working directory arguments.
pub fn resolve(
    file: Option<PathBuf>,
    working_dir: Option<PathBuf>,
) -> Result<ResolvedInput, InputError> {
    let wd = working_dir.unwrap_or_else(|| PathBuf::from("."));
    if !wd.is_dir() {
        return Err(InputError::new(format!(
            "Working directory '{}' does not exist.",
            wd.display()
        )));
    }
    let wd = wd
        .canonicalize()
        .map_err(|e| InputError::new(format!("resolving working directory: {e}")))?;

    let (config_path, chosen_name) = match file {
        Some(f) => {
            let ext_ok = f
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"))
                .unwrap_or(false);
            if !ext_ok {
                return Err(InputError::new(format!(
                    "Pipeline file '{}' must have a .yml or .yaml extension.",
                    f.display()
                )));
            }
            let path = if f.is_absolute() {
                f.clone()
            } else {
                wd.join(&f)
            };
            if !path.is_file() {
                return Err(InputError::new(format!(
                    "Pipeline file '{}' does not exist.",
                    path.display()
                )));
            }
            let name = file_name_of(&path);
            (path, name)
        }
        None => {
            let found = DEFAULT_NAMES
                .iter()
                .map(|n| wd.join(n))
                .find(|p| p.is_file());
            match found {
                Some(path) => {
                    let name = file_name_of(&path);
                    (path, name)
                }
                None => {
                    return Err(InputError::new(format!(
                        "No pipeline file found in '{}' (looked for {}).",
                        wd.display(),
                        DEFAULT_NAMES.join(", ")
                    )));
                }
            }
        }
    };

    let yaml = std::fs::read_to_string(&config_path)
        .map_err(|e| InputError::new(format!("reading {}: {e}", config_path.display())))?;

    Ok(ResolvedInput {
        working_directory: wd,
        chosen_name,
        yaml,
    })
}

fn file_name_of(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

#[derive(Deserialize)]
struct Peek {
    name: Option<String>,
    stages: Option<indexmap::IndexMap<String, serde::de::IgnoredAny>>,
}

fn peek(yaml: &str) -> Option<Peek> {
    serde_yaml_ng::from_str(yaml).ok()
}

/// Best-effort pipeline name for the header (non-authoritative; `None` on any parse failure).
pub fn peek_name(yaml: &str) -> Option<String> {
    peek(yaml)
        .and_then(|p| p.name)
        .filter(|n| !n.trim().is_empty())
}

/// Best-effort configured stage names, in order (empty on any parse failure).
pub fn peek_stages(yaml: &str) -> Vec<String> {
    peek(yaml)
        .and_then(|p| p.stages)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn prefers_release_yml_over_release_yaml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.yml"), "name: a\n").unwrap();
        fs::write(dir.path().join("release.yaml"), "name: b\n").unwrap();
        let r = resolve(None, Some(dir.path().to_path_buf())).unwrap();
        assert_eq!(r.chosen_name, "release.yml");
    }

    #[test]
    fn falls_back_to_release_yaml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.yaml"), "name: b\n").unwrap();
        let r = resolve(None, Some(dir.path().to_path_buf())).unwrap();
        assert_eq!(r.chosen_name, "release.yaml");
    }

    #[test]
    fn moonlit_yml_is_not_auto_detected() {
        // The old engine's filename. Users who still have one can pass it with -f.
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("moonlit.yml"), "name: b\n").unwrap();
        assert!(
            resolve(None, Some(dir.path().to_path_buf())).is_err(),
            "moonlit.yml must not be discovered automatically"
        );
    }

    #[test]
    fn missing_file_is_config_error() {
        let dir = tempdir().unwrap();
        let err = match resolve(None, Some(dir.path().to_path_buf())) {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn bad_extension_is_config_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.txt"), "name: a\n").unwrap();
        let err = match resolve(
            Some(PathBuf::from("release.txt")),
            Some(dir.path().to_path_buf()),
        ) {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn missing_working_dir_is_config_error() {
        let err = match resolve(None, Some(PathBuf::from("/no/such/dir/xyzzy"))) {
            Ok(_) => panic!("expected error"),
            Err(e) => e,
        };
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn peek_reads_name_and_stages_in_order() {
        let yaml = "name: My Pipe\nstages:\n  analyze:\n    - name: s\n      run: p.m\n  build:\n    - name: t\n      run: p.n\n";
        assert_eq!(peek_name(yaml), Some("My Pipe".to_string()));
        assert_eq!(peek_stages(yaml), vec!["analyze", "build"]);
    }

    #[test]
    fn peek_is_best_effort_on_garbage() {
        assert_eq!(peek_name("::: not yaml :::"), None);
        assert!(peek_stages("::: not yaml :::").is_empty());
    }
}
