use std::path::{Path, PathBuf};

#[derive(Debug, serde::Deserialize)]
struct RawManifest {
    package: RawPackage,
    lib: Option<RawLib>,
}
#[derive(Debug, serde::Deserialize)]
struct RawPackage {
    name: String,
}
#[derive(Debug, serde::Deserialize)]
struct RawLib {
    #[serde(rename = "crate-type")]
    crate_type: Option<Vec<String>>,
}

#[derive(Debug, PartialEq)]
pub struct PluginManifest {
    pub name: String,
    pub is_cdylib: bool,
}

pub fn parse_manifest(text: &str) -> Result<PluginManifest, String> {
    let raw: RawManifest = toml::from_str(text).map_err(|e| format!("bad Cargo.toml: {e}"))?;
    let is_cdylib = raw
        .lib
        .and_then(|l| l.crate_type)
        .map(|types| types.iter().any(|t| t == "cdylib"))
        .unwrap_or(false);
    Ok(PluginManifest {
        name: raw.package.name,
        is_cdylib,
    })
}

pub fn artifact_path(target_dir: &Path, lib_name: &str, release: bool) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    target_dir
        .join("wasm32-wasip2")
        .join(profile)
        .join(format!("{}.wasm", lib_name.replace('-', "_")))
}

#[derive(Debug, PartialEq)]
pub struct BuildLayout {
    pub target_dir: PathBuf,
    pub lib_name: String,
}

pub fn resolve_layout(crate_dir: &Path) -> Result<BuildLayout, String> {
    let out = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .output()
        .map_err(|e| format!("could not run cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("could not parse cargo metadata: {e}"))?;
    parse_layout(&meta, crate_dir)
}

pub fn parse_layout(meta: &serde_json::Value, crate_dir: &Path) -> Result<BuildLayout, String> {
    let target_dir = meta
        .get("target_directory")
        .and_then(|v| v.as_str())
        .ok_or("cargo metadata has no target_directory")?;

    let wanted = std::fs::canonicalize(crate_dir.join("Cargo.toml"))
        .unwrap_or_else(|_| crate_dir.join("Cargo.toml"));
    let packages = meta
        .get("packages")
        .and_then(|v| v.as_array())
        .ok_or("cargo metadata has no packages")?;
    let package = packages
        .iter()
        .find(|p| {
            p.get("manifest_path")
                .and_then(|v| v.as_str())
                .map(|m| {
                    let listed = Path::new(m);
                    std::fs::canonicalize(listed).unwrap_or_else(|_| listed.to_path_buf()) == wanted
                })
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            format!(
                "cargo metadata lists no package whose manifest is {}",
                wanted.display()
            )
        })?;

    let lib_name = package
        .get("targets")
        .and_then(|v| v.as_array())
        .and_then(|targets| {
            targets.iter().find(|t| {
                t.get("kind")
                    .and_then(|k| k.as_array())
                    .map(|kinds| kinds.iter().any(|k| k.as_str() == Some("cdylib")))
                    .unwrap_or(false)
            })
        })
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("the crate declares no cdylib target")?;

    Ok(BuildLayout {
        target_dir: PathBuf::from(target_dir),
        lib_name: lib_name.to_string(),
    })
}

pub fn wasm_target_installed() -> bool {
    let Ok(out) = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
    else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let sysroot = String::from_utf8_lossy(&out.stdout);
    Path::new(sysroot.trim())
        .join("lib/rustlib/wasm32-wasip2")
        .is_dir()
}

use crate::cli::PluginBuildArgs;

pub fn run(args: PluginBuildArgs) -> i32 {
    let crate_dir = args
        .manifest_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let manifest_file = crate_dir.join("Cargo.toml");
    let text = match std::fs::read_to_string(&manifest_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", manifest_file.display());
            return 2;
        }
    };
    let manifest = match parse_manifest(&text) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    if !manifest.is_cdylib {
        eprintln!(
            "error: {} is not a plugin crate (needs [lib] crate-type = [\"cdylib\"])",
            crate_dir.display()
        );
        return 2;
    }

    if !wasm_target_installed() {
        eprintln!("error: the wasm32-wasip2 target is not installed");
        eprintln!("  fix: rustup target add wasm32-wasip2");
        return 2;
    }

    let mut cmd = cargo_build_command(&crate_dir, args.release);
    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to run cargo: {e}");
            return 1;
        }
    };
    if !status.success() {
        eprintln!("error: cargo build failed");
        return 4;
    }

    let layout = match resolve_layout(&crate_dir) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    let artifact = artifact_path(&layout.target_dir, &layout.lib_name, args.release);
    let bytes = match std::fs::read(&artifact) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "error: build produced no artifact at {}: {e}",
                artifact.display()
            );
            return 1;
        }
    };
    match super::wasm::is_component(&bytes) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "error: {} is a core module, not a component",
                artifact.display()
            );
            return 1;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    }

    println!("✔ built {} → {}", manifest.name, artifact.display());
    0
}

fn cargo_build_command(crate_dir: &Path, release: bool) -> std::process::Command {
    let mut cmd = std::process::Command::new("cargo");
    cmd.current_dir(crate_dir)
        .args(["build", "--target", "wasm32-wasip2"])
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER");
    if release {
        cmd.arg("--release");
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_does_not_inherit_host_rustflags() {
        let cmd = cargo_build_command(Path::new("/tmp/plugin"), false);
        let removed: Vec<_> = cmd
            .get_envs()
            .filter(|(_, v)| v.is_none())
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .collect();
        for var in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_RUSTFLAGS",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
        ] {
            assert!(
                removed.contains(&var.to_string()),
                "{var} should be cleared for the wasm build, got {removed:?}"
            );
        }
    }

    #[test]
    fn build_command_targets_wasm_and_honours_release() {
        let debug = cargo_build_command(Path::new("/tmp/plugin"), false);
        let args: Vec<_> = debug.get_args().map(|a| a.to_string_lossy()).collect();
        assert_eq!(args, ["build", "--target", "wasm32-wasip2"]);

        let release = cargo_build_command(Path::new("/tmp/plugin"), true);
        let args: Vec<_> = release.get_args().map(|a| a.to_string_lossy()).collect();
        assert_eq!(args, ["build", "--target", "wasm32-wasip2", "--release"]);
    }

    #[test]
    fn parses_a_cdylib_plugin() {
        let text = r#"
            [package]
            name = "git"
            [lib]
            crate-type = ["cdylib"]
        "#;
        assert_eq!(
            parse_manifest(text).unwrap(),
            PluginManifest {
                name: "git".to_string(),
                is_cdylib: true
            }
        );
    }

    #[test]
    fn detects_non_cdylib() {
        let text = "[package]\nname = \"tool\"\n";
        let m = parse_manifest(text).unwrap();
        assert_eq!(m.name, "tool");
        assert!(!m.is_cdylib);
    }

    #[test]
    fn rejects_garbage_manifest() {
        assert!(parse_manifest("not = valid = toml").is_err());
    }

    #[test]
    fn artifact_path_uses_underscores_and_profile() {
        let p = artifact_path(Path::new("/x/target"), "my-plugin", true);
        assert_eq!(
            p,
            PathBuf::from("/x/target/wasm32-wasip2/release/my_plugin.wasm")
        );
        let d = artifact_path(Path::new("/x/target"), "my-plugin", false);
        assert!(d.ends_with("target/wasm32-wasip2/debug/my_plugin.wasm"));
    }

    #[test]
    fn parse_layout_uses_the_workspace_target_dir_and_the_lib_name() {
        let meta = serde_json::json!({
            "target_directory": "/ws/target",
            "packages": [
                {
                    "name": "moonlit-plugin-other",
                    "manifest_path": "/ws/other/Cargo.toml",
                    "targets": [{ "kind": ["cdylib"], "name": "other" }]
                },
                {
                    "name": "moonlit-plugin-git",
                    "manifest_path": "/ws/git/Cargo.toml",
                    "targets": [{ "kind": ["cdylib"], "name": "git" }]
                }
            ]
        });
        let layout = parse_layout(&meta, Path::new("/ws/git")).unwrap();
        assert_eq!(layout.target_dir, PathBuf::from("/ws/target"));
        assert_eq!(layout.lib_name, "git");
        assert_eq!(
            artifact_path(&layout.target_dir, &layout.lib_name, true),
            PathBuf::from("/ws/target/wasm32-wasip2/release/git.wasm")
        );
    }

    #[test]
    fn parse_layout_errors_rather_than_guessing_when_no_package_matches() {
        let meta = serde_json::json!({
            "target_directory": "/ws/target",
            "packages": [{
                "name": "moonlit-plugin-docker",
                "manifest_path": "/ws/docker/Cargo.toml",
                "targets": [{ "kind": ["cdylib"], "name": "docker" }]
            }]
        });
        let err = parse_layout(&meta, Path::new("/ws/git")).unwrap_err();
        assert!(err.contains("no package"), "unexpected error: {err}");
    }

    #[test]
    fn parse_layout_rejects_a_crate_with_no_cdylib_target() {
        let meta = serde_json::json!({
            "target_directory": "/ws/target",
            "packages": [{
                "name": "plain",
                "manifest_path": "/ws/plain/Cargo.toml",
                "targets": [{ "kind": ["lib"], "name": "plain" }]
            }]
        });
        assert!(parse_layout(&meta, Path::new("/ws/plain")).is_err());
    }
}
