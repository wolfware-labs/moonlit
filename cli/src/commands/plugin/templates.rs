//! Embedded scaffold templates. Placeholders are `{token}`; substitution is a
//! single-pass token match (no format!), so literal braces in Rust/TOML
//! bodies need no escaping.

use std::path::PathBuf;

use super::scaffold::ScaffoldValues;

const CARGO_TOML: &str = r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
rust-version = "1.97"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
moonlit-plugin-sdk = {sdk_dep}
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "s"
"#;

const LIB_RS: &str = r#"use moonlit_plugin_sdk::prelude::*;

/// Config for the `greet` middleware. Fields bind from step config with
/// string-coercion; unknown keys are ignored.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct GreetConfig {
    /// Who to greet; defaults to "world" when empty.
    name: String,
}

#[derive(Default)]
struct Greet;

impl Middleware for Greet {
    const NAME: &'static str = "greet";
    const DESCRIPTION: &'static str = "logs and returns a greeting";
    type Config = GreetConfig;

    fn execute(&self, ctx: &Context, cfg: Self::Config) -> MiddlewareResult {
        let who = if cfg.name.is_empty() {
            "world".to_string()
        } else {
            cfg.name
        };
        ctx.log_info(&format!("greeting {who}"));
        MiddlewareResult::success_with(|o| {
            o.set("greeting", format!("hello, {who}"));
        })
    }
}

moonlit_plugin! {
    name: "{name}",
    middlewares: [Greet],
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    #[test]
    fn greet_greets_the_configured_name() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/work".to_string(), "test".to_string());
        let result = run(&Greet, &ctx, GreetConfig { name: "moonlit".to_string() });
        assert!(result.is_success());
        assert!(host.logs().iter().any(|(_, m)| m.contains("moonlit")));
    }
}
"#;

const PLUGIN_TOML: &str = r#"name = "{name}"
namespace = "{namespace}"
description = "{description}"
license = "{license}"
"#;

const README: &str = r#"# {name}

A Moonlit plugin.

## Build

    moonlit plugin build --release

## Test

    cargo test

## Inspect

    moonlit plugin inspect target/wasm32-wasip2/release/{artifact}.wasm
"#;

const GITIGNORE: &str = "/target\n";

/// Substitute `{token}` placeholders in one pass. Each `{` is matched against
/// the known tokens; a match emits the value and skips the token in the
/// template, so an inserted value is never re-scanned (no double substitution)
/// and unknown brace sequences in the body (e.g. Rust `format!` args) pass
/// through untouched.
fn substitute(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    'outer: while i < template.len() {
        if template.as_bytes()[i] == b'{' {
            for (token, value) in vars {
                if template[i..].starts_with(token) {
                    out.push_str(value);
                    i += token.len();
                    continue 'outer;
                }
            }
        }
        let ch = template[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Render every scaffold file as (relative path, contents).
pub fn render_all(v: &ScaffoldValues) -> Vec<(PathBuf, String)> {
    let artifact = v.name.replace('-', "_");
    let vars = [
        ("{name}", v.name.as_str()),
        ("{namespace}", v.namespace.as_str()),
        ("{description}", v.description.as_str()),
        ("{license}", v.license.as_str()),
        ("{sdk_dep}", v.sdk_dep.as_str()),
        ("{artifact}", artifact.as_str()),
    ];
    vec![
        (PathBuf::from("Cargo.toml"), substitute(CARGO_TOML, &vars)),
        (PathBuf::from("src/lib.rs"), substitute(LIB_RS, &vars)),
        (
            PathBuf::from("moonlit-plugin.toml"),
            substitute(PLUGIN_TOML, &vars),
        ),
        (PathBuf::from("README.md"), substitute(README, &vars)),
        (PathBuf::from(".gitignore"), GITIGNORE.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> ScaffoldValues {
        ScaffoldValues {
            name: "my-plugin".to_string(),
            namespace: "acme".to_string(),
            description: "does things".to_string(),
            license: "Apache-2.0".to_string(),
            sdk_dep: "\"0.1.0\"".to_string(),
        }
    }

    #[test]
    fn renders_all_five_files() {
        let files = render_all(&values());
        let names: Vec<_> = files.iter().map(|(p, _)| p.to_str().unwrap()).collect();
        assert!(names.contains(&"Cargo.toml"));
        assert!(names.contains(&"src/lib.rs"));
        assert!(names.contains(&"moonlit-plugin.toml"));
        assert!(names.contains(&"README.md"));
        assert!(names.contains(&".gitignore"));
    }

    #[test]
    fn cargo_toml_substitutes_name_and_dep() {
        let files = render_all(&values());
        let cargo = &files
            .iter()
            .find(|(p, _)| p.to_str() == Some("Cargo.toml"))
            .unwrap()
            .1;
        assert!(cargo.contains("name = \"my-plugin\""));
        assert!(cargo.contains("moonlit-plugin-sdk = \"0.1.0\""));
    }

    #[test]
    fn lib_rs_uses_plugin_name_and_no_placeholder_remains() {
        let files = render_all(&values());
        let lib = &files
            .iter()
            .find(|(p, _)| p.to_str() == Some("src/lib.rs"))
            .unwrap()
            .1;
        assert!(lib.contains("name: \"my-plugin\""));
        assert!(!lib.contains("{name}"));
    }

    #[test]
    fn substitute_does_not_rescan_inserted_values() {
        // A user value that itself contains a later token must survive literally.
        let vars = [
            ("{description}", "uses {license} here"),
            ("{license}", "MIT"),
        ];
        assert_eq!(
            substitute("d={description}\nl={license}\n", &vars),
            "d=uses {license} here\nl=MIT\n"
        );
    }

    #[test]
    fn substitute_passes_unknown_brace_tokens_through() {
        // Unknown tokens (e.g. Rust format args in a template body) are untouched.
        assert_eq!(substitute("x {who} y", &[("{name}", "n")]), "x {who} y");
    }

    #[test]
    fn readme_uses_underscore_artifact_name() {
        let files = render_all(&values());
        let readme = &files
            .iter()
            .find(|(p, _)| p.to_str() == Some("README.md"))
            .unwrap()
            .1;
        assert!(readme.contains("my_plugin.wasm"));
    }
}
