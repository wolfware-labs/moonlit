//! `moonlit plugin build` helpers + command.

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

/// The subset of a plugin's `Cargo.toml` that `build` needs.
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

/// Where cargo drops the component: cdylib names use `_` for `-`.
pub fn artifact_path(crate_dir: &Path, pkg_name: &str, release: bool) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    crate_dir
        .join("target/wasm32-wasip2")
        .join(profile)
        .join(format!("{}.wasm", pkg_name.replace('-', "_")))
}

/// True if the `wasm32-wasip2` target is installed in the active toolchain.
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let p = artifact_path(Path::new("/x"), "my-plugin", true);
        assert_eq!(
            p,
            PathBuf::from("/x/target/wasm32-wasip2/release/my_plugin.wasm")
        );
        let d = artifact_path(Path::new("/x"), "my-plugin", false);
        assert!(d.ends_with("target/wasm32-wasip2/debug/my_plugin.wasm"));
    }
}
