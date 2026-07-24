//! `moonlit plugin publish <ref>` — introspect a built component and push it as an OCI artifact.

use std::path::PathBuf;

use moonlit_engine::publish::{PublishMeta, new_push_client, publish_plugin};

use crate::cli::{OutputMode, PluginPublishArgs};
use crate::render::resolve_mode;

/// Best-effort: the resolved `moonlit-plugin-sdk` version from a crate's `Cargo.lock`.
pub fn sdk_version_from_lock(lock_text: &str) -> Option<String> {
    let doc: toml::Value = toml::from_str(lock_text).ok()?;
    let packages = doc.get("package")?.as_array()?;
    for pkg in packages {
        if pkg.get("name").and_then(|n| n.as_str()) == Some("moonlit-plugin-sdk") {
            return pkg
                .get("version")
                .and_then(|v| v.as_str())
                .map(str::to_string);
        }
    }
    None
}

#[derive(serde::Deserialize)]
struct CrateFacts {
    package: CratePackage,
}
#[derive(serde::Deserialize)]
struct CratePackage {
    repository: Option<String>,
    license: Option<String>,
}

/// Best-effort `(source, licenses)` from a crate's `Cargo.toml`.
fn read_crate_facts(crate_dir: &std::path::Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(crate_dir.join("Cargo.toml")) else {
        return (None, None);
    };
    match toml::from_str::<CrateFacts>(&text) {
        Ok(f) => (f.package.repository, f.package.license),
        Err(_) => (None, None),
    }
}

pub async fn run(output: Option<OutputMode>, args: PluginPublishArgs) -> i32 {
    let crate_dir = args
        .manifest_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    // Resolve the component bytes: --file, else the crate's release artifact.
    let file = match &args.file {
        Some(f) => f.clone(),
        None => {
            let manifest_file = crate_dir.join("Cargo.toml");
            let text = match std::fs::read_to_string(&manifest_file) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("error: cannot read {}: {e}", manifest_file.display());
                    eprintln!("  fix: run from the plugin crate directory or pass --file");
                    return 2;
                }
            };
            let manifest = match super::build::parse_manifest(&text) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 2;
                }
            };
            super::build::artifact_path(&crate_dir, &manifest.name, true)
        }
    };
    let bytes = match std::fs::read(&file) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: no built component at {} ({e})", file.display());
            eprintln!("  fix: run `moonlit plugin build --release` or pass --file");
            return 2;
        }
    };

    match super::wasm::is_component(&bytes) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "error: '{}' is a core wasm module, not a WASI-P2 component (build with `moonlit plugin build`)",
                file.display()
            );
            return 2;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    }
    if let Err(e) = super::wasm::validate(&bytes) {
        eprintln!("error: {e}");
        return 2;
    }

    let (meta, mws) = match super::introspect::introspect(&bytes).await {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: failed to instantiate component: {e}");
            return 3;
        }
    };

    let (source, licenses) = read_crate_facts(&crate_dir);
    let sdk_version = std::fs::read_to_string(crate_dir.join("Cargo.lock"))
        .ok()
        .and_then(|t| sdk_version_from_lock(&t));

    let publish_meta = PublishMeta {
        plugin_name: meta.name.clone(),
        version: meta.version.clone(),
        description: meta.description.clone(),
        source,
        licenses,
        middlewares: mws.iter().map(|m| m.name.clone()).collect(),
        sdk_version,
    };

    let home = dirs::home_dir().unwrap_or_default();
    let client = new_push_client();
    match publish_plugin(&args.reference, bytes, publish_meta, &home, &client).await {
        Ok(outcome) => {
            let stdout_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
            match resolve_mode(output, stdout_tty) {
                OutputMode::Json => println!(
                    "{}",
                    serde_json::json!({
                        "reference": outcome.reference,
                        "digest": outcome.digest,
                        "size": outcome.size,
                    })
                ),
                _ => println!(
                    "Published {}  {}  ({} bytes)",
                    outcome.reference, outcome.digest, outcome.size
                ),
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            3
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_sdk_version_in_lock() {
        let lock = r#"
[[package]]
name = "some-dep"
version = "1.2.3"

[[package]]
name = "moonlit-plugin-sdk"
version = "0.4.1"
"#;
        assert_eq!(sdk_version_from_lock(lock), Some("0.4.1".to_string()));
    }

    #[test]
    fn missing_sdk_package_yields_none() {
        let lock = "[[package]]\nname = \"x\"\nversion = \"1.0.0\"\n";
        assert_eq!(sdk_version_from_lock(lock), None);
    }
}
