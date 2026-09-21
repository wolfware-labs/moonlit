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
