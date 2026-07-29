//! `gitlab write-variables` — write/append dotenv `key=value` lines to a file in
//! the working directory (for GitLab CI `artifacts:reports:dotenv`). The file is
//! inside the engine's preopened working dir, so this writes via `std::fs`
//! directly — no process capability needed.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use moonlit_sdk::prelude::*;
use regex::Regex;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WriteVariablesConfig {
    output: BTreeMap<String, String>,
    environment: BTreeMap<String, String>,
    file: String,
}

impl Default for WriteVariablesConfig {
    fn default() -> Self {
        Self {
            output: BTreeMap::new(),
            environment: BTreeMap::new(),
            file: "moonlit.env".to_string(),
        }
    }
}

/// Render the merged map to dotenv lines. Rejects keys not matching
/// `[A-Za-z_][A-Za-z0-9_]*`; escapes newlines/CRs to literal `\n`/`\r` so a value
/// can never inject a spurious dotenv line (GitLab dotenv is single-line).
fn render(map: &BTreeMap<String, String>) -> Result<String, String> {
    let key_re = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap();
    let mut s = String::new();
    for (k, v) in map {
        if !key_re.is_match(k) {
            return Err(format!(
                "Invalid variable name '{k}' (must match [A-Za-z_][A-Za-z0-9_]*)."
            ));
        }
        let escaped = v.replace('\r', "\\r").replace('\n', "\\n");
        s.push_str(&format!("{k}={escaped}\n"));
    }
    Ok(s)
}

/// Resolve the target file path. Under wasm the working dir IS the preopen (`.`),
/// so a relative path is correct; under native tests we join the (host) working dir.
#[cfg(target_arch = "wasm32")]
fn resolve_path(_working_dir: &str, file: &str) -> PathBuf {
    PathBuf::from(file)
}
#[cfg(not(target_arch = "wasm32"))]
fn resolve_path(working_dir: &str, file: &str) -> PathBuf {
    Path::new(working_dir).join(file)
}

#[derive(Default)]
pub struct WriteVariables;

impl Middleware for WriteVariables {
    const NAME: &'static str = "write-variables";
    const DESCRIPTION: &'static str =
        "write step outputs / env to a dotenv file for artifacts:reports:dotenv";
    type Config = WriteVariablesConfig;

    fn execute(&self, ctx: &Context, cfg: WriteVariablesConfig) -> MiddlewareResult {
        // Merge: output first, then environment (environment wins on collision).
        let mut merged: BTreeMap<String, String> = cfg.output.clone();
        for (k, v) in &cfg.environment {
            if merged.contains_key(k) {
                ctx.log_warn(&format!(
                    "Variable '{k}' defined in both output and environment; using the environment value."
                ));
            }
            merged.insert(k.clone(), v.clone());
        }
        if merged.is_empty() {
            return MiddlewareResult::success();
        }
        // Path hardening: relative, within the working directory.
        if Path::new(&cfg.file).is_absolute()
            || Path::new(&cfg.file)
                .components()
                .any(|c| matches!(c, Component::ParentDir))
        {
            return MiddlewareResult::failure(format!(
                "Invalid file path '{}' (must be a relative path within the working directory).",
                cfg.file
            ));
        }
        let content = match render(&merged) {
            Ok(c) => c,
            Err(e) => return MiddlewareResult::failure(e),
        };
        let path = resolve_path(ctx.working_dir(), &cfg.file);
        let mut f = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) => {
                return MiddlewareResult::failure(format!("Failed to write '{}': {e}", cfg.file))
            }
        };
        if let Err(e) = f.write_all(content.as_bytes()) {
            return MiddlewareResult::failure(format!("Failed to write '{}': {e}", cfg.file));
        }
        MiddlewareResult::success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::{run, MockHost};

    fn cfg(
        output: &[(&str, &str)],
        environment: &[(&str, &str)],
        file: &str,
    ) -> WriteVariablesConfig {
        WriteVariablesConfig {
            output: output
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            environment: environment
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            file: file.to_string(),
        }
    }

    #[test]
    fn writes_sorted_dotenv_lines() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(
            &WriteVariables,
            &ctx,
            cfg(&[("VERSION", "1.2.3"), ("NAME", "app")], &[], "moonlit.env"),
        )
        .into_wit();
        assert!(w.successful);
        let written = std::fs::read_to_string(dir.path().join("moonlit.env")).unwrap();
        assert_eq!(written, "NAME=app\nVERSION=1.2.3\n"); // BTreeMap sorts keys
    }

    #[test]
    fn environment_wins_on_collision_and_warns() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(
            &WriteVariables,
            &ctx,
            cfg(&[("A", "1")], &[("A", "2"), ("B", "3")], "moonlit.env"),
        )
        .into_wit();
        assert!(w.successful);
        let written = std::fs::read_to_string(dir.path().join("moonlit.env")).unwrap();
        assert_eq!(written, "A=2\nB=3\n");
        assert!(host.logs().iter().any(|(_, m)| {
            m == "Variable 'A' defined in both output and environment; using the environment value."
        }));
    }

    #[test]
    fn escapes_newlines_to_literal_backslash_n() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(
            &WriteVariables,
            &ctx,
            cfg(&[("NOTES", "line1\nline2\r")], &[], "moonlit.env"),
        )
        .into_wit();
        assert!(w.successful);
        let written = std::fs::read_to_string(dir.path().join("moonlit.env")).unwrap();
        assert_eq!(written, "NOTES=line1\\nline2\\r\n");
    }

    #[test]
    fn rejects_invalid_key() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(
            &WriteVariables,
            &ctx,
            cfg(&[("bad key", "x")], &[], "moonlit.env"),
        )
        .into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Invalid variable name 'bad key' (must match [A-Za-z_][A-Za-z0-9_]*).")
        );
        assert!(!dir.path().join("moonlit.env").exists());
    }

    #[test]
    fn rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(&WriteVariables, &ctx, cfg(&[("A", "1")], &[], "../evil")).into_wit();
        assert!(!w.successful);
        assert_eq!(w.error_message.as_deref(), Some("Invalid file path '../evil' (must be a relative path within the working directory)."));
    }

    #[test]
    fn custom_file_name_is_honored() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(&WriteVariables, &ctx, cfg(&[("A", "1")], &[], "custom.env")).into_wit();
        assert!(w.successful);
        assert!(dir.path().join("custom.env").exists());
    }

    #[test]
    fn empty_maps_succeed_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        let w = run(&WriteVariables, &ctx, cfg(&[], &[], "moonlit.env")).into_wit();
        assert!(w.successful);
        assert!(!dir.path().join("moonlit.env").exists());
    }

    #[test]
    fn default_file_is_moonlit_env() {
        assert_eq!(WriteVariablesConfig::default().file, "moonlit.env");
    }

    #[test]
    fn second_write_appends_rather_than_truncates() {
        // Two invocations against the same file must accumulate — proves
        // OpenOptions uses append(true), not truncate.
        let dir = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, dir.path().to_str().unwrap().into(), "s".into());
        run(
            &WriteVariables,
            &ctx,
            cfg(&[("A", "1")], &[], "moonlit.env"),
        )
        .into_wit();
        let w = run(
            &WriteVariables,
            &ctx,
            cfg(&[("B", "2")], &[], "moonlit.env"),
        )
        .into_wit();
        assert!(w.successful);
        let written = std::fs::read_to_string(dir.path().join("moonlit.env")).unwrap();
        assert_eq!(written, "A=1\nB=2\n");
    }
}
