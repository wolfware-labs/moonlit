//! `dotnet pack` — pack a project into a `.nupkg`, scanning the output dir for the result.

use crate::dotnet::{dotnet, exit_phrase, prepare_output_dir, resolve};
use crate::version::{assembly_or_file_version, informational_version, package_version};
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;
use std::path::{Path, PathBuf};

/// From a prepared output dir (`out_dir`, whose logical relative path is `out_rel`), pick
/// the produced package: the alphabetically-first `*.nupkg` (sorted for determinism).
/// Returns the working-dir-relative packagePath and an optional warning (when >1 file was
/// produced). `Err(())` when no package was produced.
fn selected_package(out_dir: &Path, out_rel: &str) -> Result<(String, Option<String>), ()> {
    let mut nupkgs: Vec<PathBuf> = std::fs::read_dir(out_dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("nupkg"))
                .collect()
        })
        .unwrap_or_default();
    nupkgs.sort();
    let first = match nupkgs.first() {
        None => return Err(()),
        Some(p) => p,
    };
    let filename = first
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let path = format!("{out_rel}/{filename}");
    let warning = if nupkgs.len() > 1 {
        let stem = first.file_stem().unwrap_or_default().to_string_lossy();
        Some(format!(
            "Multiple .nupkg files were created. Using the first one: {stem}"
        ))
    } else {
        None
    };
    Ok((path, warning))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PackConfig {
    pub project: String,
    pub version: Option<String>,
    pub assembly_version: Option<String>,
    pub file_version: Option<String>,
    pub informational_version: Option<String>,
    pub package_version: Option<String>,
    pub configuration: String,
    pub no_build: bool,
    pub no_restore: bool,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            project: String::new(),
            version: None,
            assembly_version: None,
            file_version: None,
            informational_version: None,
            package_version: None,
            configuration: "Release".to_string(),
            no_build: false,
            no_restore: false,
        }
    }
}

#[derive(Default)]
pub struct Pack;

impl Middleware for Pack {
    const NAME: &'static str = "pack";
    const DESCRIPTION: &'static str = "pack a .NET project into a NuGet package";
    type Config = PackConfig;

    fn execute(&self, ctx: &Context, cfg: PackConfig) -> MiddlewareResult {
        let proj_path = resolve(ctx.working_dir(), &cfg.project);
        if !proj_path.is_file() {
            return MiddlewareResult::failure(format!(
                "Project file not found at path: {}",
                proj_path.display()
            ));
        }
        let stem = Path::new(&cfg.project)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let out_rel = format!(".moonlit/dotnet/{stem}");
        let out_dir = match prepare_output_dir(ctx.working_dir(), &out_rel) {
            Ok(d) => d,
            Err(e) => {
                return MiddlewareResult::failure(format!(
                    "Failed to prepare output directory: {e}"
                ))
            }
        };

        let assembly_version = match assembly_or_file_version(&cfg.assembly_version, &cfg.version) {
            Some(v) => v,
            None => return MiddlewareResult::failure(
                "AssemblyVersion could not be determined. Please specify it in the configuration or provide a valid Version.",
            ),
        };
        let file_version = match assembly_or_file_version(&cfg.file_version, &cfg.version) {
            Some(v) => v,
            None => return MiddlewareResult::failure(
                "FileVersion could not be determined. Please specify it in the configuration or provide a valid Version.",
            ),
        };
        let info_version = match informational_version(&cfg.informational_version, &cfg.version) {
            Some(v) => v,
            None => return MiddlewareResult::failure(
                "InformationalVersion could not be determined. Please specify it in the configuration or provide a valid Version.",
            ),
        };
        let package_ver = match package_version(&cfg.package_version, &cfg.version) {
            Some(v) => v,
            None => return MiddlewareResult::failure(
                "PackageVersion could not be determined. Please specify it in the configuration or provide a valid Version.",
            ),
        };

        let mut args: Vec<String> = vec![
            "pack".to_string(),
            cfg.project.clone(),
            format!("-p:AssemblyVersion={assembly_version}"),
            format!("-p:FileVersion={file_version}"),
            format!("-p:InformationalVersion={info_version}"),
            format!("-p:PackageVersion={package_ver}"),
            format!("-p:Version={package_ver}"),
            "--output".to_string(),
            out_rel.clone(),
            "--configuration".to_string(),
            cfg.configuration.clone(),
        ];
        if cfg.no_build {
            args.push("--no-build".to_string());
        }
        if cfg.no_restore {
            args.push("--no-restore".to_string());
        }

        // Run the pack; on non-zero exit (or spawn error), fail before scanning.
        match dotnet(ctx).args(args).stream(LineHandler::severity()) {
            Ok(o) if o.success() => {}
            Ok(o) => {
                return MiddlewareResult::failure(format!(
                    "Failed to pack project: {}",
                    exit_phrase(o.exit_code)
                ))
            }
            Err(e) => return MiddlewareResult::failure(format!("Failed to pack project: {e}")),
        }

        match selected_package(&out_dir, &out_rel) {
            Err(()) => MiddlewareResult::failure("No .nupkg files were created."),
            Ok((path, warning)) => {
                if let Some(w) = &warning {
                    ctx.log_warn(w);
                }
                let mut out = MiddlewareResult::success_with(|o| o.set("packagePath", path));
                if let Some(w) = warning {
                    out = out.with_warning(w);
                }
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn proj_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("App.csproj"), b"<Project/>").unwrap();
        d
    }
    fn seed_nupkgs(dir: &std::path::Path, n: usize) {
        std::fs::create_dir_all(dir).unwrap();
        for i in 0..n {
            std::fs::write(dir.join(format!("App.{i}.nupkg")), b"pkg").unwrap();
        }
    }

    // --- selected_package (pure) ---
    #[test]
    fn selected_package_none_is_err() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        assert!(selected_package(&out, ".moonlit/dotnet/App").is_err());
    }
    #[test]
    fn selected_package_single_returns_relative_path_no_warning() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("out");
        seed_nupkgs(&out, 1);
        let (path, warn) = selected_package(&out, ".moonlit/dotnet/App").unwrap();
        assert_eq!(path, ".moonlit/dotnet/App/App.0.nupkg");
        assert!(warn.is_none());
    }
    #[test]
    fn selected_package_multiple_warns_and_uses_first_sorted() {
        let d = tempfile::tempdir().unwrap();
        let out = d.path().join("out");
        seed_nupkgs(&out, 2);
        let (path, warn) = selected_package(&out, ".moonlit/dotnet/App").unwrap();
        assert_eq!(path, ".moonlit/dotnet/App/App.0.nupkg");
        assert!(warn.unwrap().contains("Multiple .nupkg files were created"));
    }

    // --- Pack middleware ---
    // MockHost's "dotnet pack" creates no file, so the post-scan fails; argv tests assert
    // the recorded command (captured during `.stream()`, before the scan) and ignore the result.
    #[test]
    fn pack_builds_expected_argv() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "pack".into());
        let cfg = PackConfig {
            project: "App.csproj".into(),
            version: Some("1.2.3-rc.1+meta".into()),
            ..Default::default()
        };
        let _ = run(&Pack, &ctx, cfg);
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[0].args,
            vec![
                "pack",
                "App.csproj",
                "-p:AssemblyVersion=1.2.3",
                "-p:FileVersion=1.2.3",
                "-p:InformationalVersion=1.2.3-rc.1+meta",
                "-p:PackageVersion=1.2.3-rc.1",
                "-p:Version=1.2.3-rc.1",
                "--output",
                ".moonlit/dotnet/App",
                "--configuration",
                "Release",
            ]
        );
    }
    #[test]
    fn pack_no_build_and_no_restore_flags_appended() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "pack".into());
        let cfg = PackConfig {
            project: "App.csproj".into(),
            version: Some("1.0.0".into()),
            no_build: true,
            no_restore: true,
            ..Default::default()
        };
        let _ = run(&Pack, &ctx, cfg);
        let cmds = host.recorded_commands();
        let a = &cmds[0].args;
        assert_eq!(
            &a[a.len() - 2..],
            &["--no-build".to_string(), "--no-restore".to_string()]
        );
    }
    #[test]
    fn pack_missing_project_fails() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "pack".into());
        let cfg = PackConfig {
            project: "nope.csproj".into(),
            version: Some("1.0.0".into()),
            ..Default::default()
        };
        let w = run(&Pack, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert!(w
            .error_message
            .unwrap()
            .starts_with("Project file not found at path:"));
    }
    #[test]
    fn pack_unresolved_assembly_version_fails() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "pack".into());
        let cfg = PackConfig {
            project: "App.csproj".into(),
            ..Default::default()
        }; // no version
        let w = run(&Pack, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("AssemblyVersion could not be determined. Please specify it in the configuration or provide a valid Version.")
        );
    }
    #[test]
    fn pack_no_nupkg_produced_fails() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]); // mock creates no file
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "pack".into());
        let cfg = PackConfig {
            project: "App.csproj".into(),
            version: Some("1.0.0".into()),
            ..Default::default()
        };
        let w = run(&Pack, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("No .nupkg files were created.")
        );
    }
}
