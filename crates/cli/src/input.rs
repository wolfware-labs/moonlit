use std::path::{Path, PathBuf};

use serde::Deserialize;

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

pub struct ResolvedInput {
    pub working_directory: PathBuf,
    pub chosen_name: String,
    pub yaml: String,
}

const DEFAULT_NAMES: [&str; 2] = ["release.yml", "release.yaml"];

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
            let path = wd.join(&f);
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

pub fn peek_name(yaml: &str) -> Option<String> {
    peek(yaml)
        .and_then(|p| p.name)
        .filter(|n| !n.trim().is_empty())
}

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

    fn err_of(r: Result<ResolvedInput, InputError>) -> InputError {
        match r {
            Ok(_) => panic!("expected an error, got a resolved input"),
            Err(e) => e,
        }
    }

    #[test]
    fn input_errors_carry_the_configuration_exit_code() {
        assert_eq!(InputError::new("anything").exit_code(), 2);
    }

    #[test]
    fn prefers_release_yml_over_release_yaml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.yml"), "name: a\n").unwrap();
        fs::write(dir.path().join("release.yaml"), "name: b\n").unwrap();
        let r = resolve(None, Some(dir.path().to_path_buf())).unwrap();
        assert_eq!(r.chosen_name, "release.yml");
        assert_eq!(r.yaml, "name: a\n");
    }

    #[test]
    fn falls_back_to_release_yaml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.yaml"), "name: b\n").unwrap();
        let r = resolve(None, Some(dir.path().to_path_buf())).unwrap();
        assert_eq!(r.chosen_name, "release.yaml");
        assert_eq!(r.yaml, "name: b\n");
    }

    #[test]
    fn moonlit_yml_is_not_auto_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("moonlit.yml"), "name: b\n").unwrap();
        let err = err_of(resolve(None, Some(dir.path().to_path_buf())));
        assert!(
            err.0.contains("No pipeline file found"),
            "moonlit.yml must not be discovered automatically, got: {}",
            err.0
        );
    }

    #[test]
    fn nothing_to_discover_reports_the_directory_and_the_names_tried() {
        let dir = tempdir().unwrap();
        let err = err_of(resolve(None, Some(dir.path().to_path_buf())));
        let canonical = dir.path().canonicalize().unwrap();
        assert!(
            err.0.contains(&canonical.display().to_string()),
            "must name the directory searched: {}",
            err.0
        );
        for name in DEFAULT_NAMES {
            assert!(
                err.0.contains(name),
                "must list `{name}` among the names tried: {}",
                err.0
            );
        }
    }

    #[test]
    fn a_named_file_that_is_absent_is_reported_as_absent() {
        let dir = tempdir().unwrap();
        let err = err_of(resolve(
            Some(PathBuf::from("nope.yml")),
            Some(dir.path().to_path_buf()),
        ));
        assert!(err.0.contains("does not exist"), "{}", err.0);
        assert!(err.0.contains("nope.yml"), "must name the file: {}", err.0);
    }

    #[test]
    fn rejects_a_named_file_whose_extension_is_neither_yml_nor_yaml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.txt"), "name: a\n").unwrap();
        let err = err_of(resolve(
            Some(PathBuf::from("release.txt")),
            Some(dir.path().to_path_buf()),
        ));
        assert!(
            err.0.contains("must have a .yml or .yaml extension"),
            "must be rejected for its extension, not for anything else: {}",
            err.0
        );
    }

    #[test]
    fn the_extension_gate_ignores_case() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("pipeline.YML"), "name: shouty\n").unwrap();
        let r = resolve(
            Some(PathBuf::from("pipeline.YML")),
            Some(dir.path().to_path_buf()),
        )
        .unwrap();
        assert_eq!(r.yaml, "name: shouty\n");
    }

    #[test]
    fn an_absolute_named_file_is_read_from_outside_the_working_directory() {
        let wd = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        let outside = elsewhere.path().join("other.yml");
        fs::write(&outside, "name: outside\n").unwrap();

        let r = resolve(Some(outside), Some(wd.path().to_path_buf())).unwrap();
        assert_eq!(r.chosen_name, "other.yml");
        assert_eq!(r.yaml, "name: outside\n");
        assert_eq!(
            r.working_directory,
            wd.path().canonicalize().unwrap(),
            "an absolute -f must not move the working directory"
        );
    }

    #[test]
    fn a_missing_working_directory_is_named_in_the_error() {
        let err = err_of(resolve(None, Some(PathBuf::from("/no/such/dir/xyzzy"))));
        assert!(err.0.contains("Working directory"), "{}", err.0);
        assert!(
            err.0.contains("/no/such/dir/xyzzy"),
            "must name the directory: {}",
            err.0
        );
    }

    #[test]
    fn the_working_directory_is_canonicalized() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("release.yml"), "name: a\n").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let r = resolve(None, Some(dir.path().join("sub").join(".."))).unwrap();
        assert_eq!(r.working_directory, dir.path().canonicalize().unwrap());
        assert!(
            !r.working_directory.to_string_lossy().contains(".."),
            "the engine receives this path; it must not carry traversal segments: {}",
            r.working_directory.display()
        );
    }

    #[test]
    fn peek_reads_the_name_and_the_stages_in_declaration_order() {
        let yaml = "\
name: My Pipe
stages:
  verify:
    - name: s
      run: p.m
  analyze:
    - name: t
      run: p.n
  build:
    - name: u
      run: p.o
  deploy:
    - name: v
      run: p.p
  announce:
    - name: w
      run: p.q
";
        assert_eq!(peek_name(yaml), Some("My Pipe".to_string()));
        assert_eq!(
            peek_stages(yaml),
            vec!["verify", "analyze", "build", "deploy", "announce"]
        );
    }

    #[test]
    fn peek_name_treats_a_blank_name_as_absent() {
        assert_eq!(peek_name("name: ''\nstages: {}\n"), None);
        assert_eq!(peek_name("name: '   '\nstages: {}\n"), None);
        assert_eq!(peek_name("name: \"\\t\"\nstages: {}\n"), None);
    }

    #[test]
    fn peek_is_best_effort_on_yaml_of_the_wrong_shape() {
        for yaml in ["- a\n- b\n", "just a scalar\n", "42\n"] {
            assert_eq!(peek_name(yaml), None, "input: {yaml:?}");
            assert!(peek_stages(yaml).is_empty(), "input: {yaml:?}");
        }
    }

    #[test]
    fn peek_is_best_effort_on_unparseable_yaml() {
        assert_eq!(peek_name("::: not yaml :::"), None);
        assert!(peek_stages("::: not yaml :::").is_empty());
    }

    #[test]
    fn peek_tolerates_a_pipeline_that_declares_no_stages() {
        assert_eq!(peek_name("name: bare\n"), Some("bare".to_string()));
        assert!(peek_stages("name: bare\n").is_empty());
    }
}
