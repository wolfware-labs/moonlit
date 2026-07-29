//! Shared `dotnet` command helpers: cwd-seeded builder, project-path resolution
//! against the wasm preopen, and the uniform non-zero-exit phrase.

use moonlit_sdk::process::Command;
use moonlit_sdk::Context;
use std::path::{Path, PathBuf};

/// A `dotnet` command pre-seeded with the working directory as cwd.
pub fn dotnet<'a>(ctx: &Context<'a>) -> Command<'a> {
    ctx.command("dotnet").cwd(ctx.working_dir())
}

/// Resolve a config path against the working directory. Under wasm the preopen IS
/// the working dir (`.`), so a relative path is correct; native tests join the host dir.
#[cfg(target_arch = "wasm32")]
pub fn resolve(_working_dir: &str, p: &str) -> PathBuf {
    PathBuf::from(p)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn resolve(working_dir: &str, p: &str) -> PathBuf {
    std::path::Path::new(working_dir).join(p)
}

/// The uniform non-zero-exit failure phrase (matches 1.x `DotnetClient`).
pub fn exit_phrase(code: i32) -> String {
    format!("Dotnet command failed with exit code {code}")
}

/// A filesystem-safe, collision-resistant slug for a project's output directory: the
/// project's relative path minus its extension, with path separators flattened to `_`.
/// Keying on the full relative path (not just the file stem) stops same-named projects in
/// different directories from sharing — and wiping — one output dir within a single run.
pub fn project_slug(project: &str) -> String {
    Path::new(project)
        .with_extension("")
        .to_string_lossy()
        .replace(['/', '\\'], "_")
        .trim_matches('_')
        .to_string()
}

/// Create a fresh (wiped) output directory `rel` under the working dir. Wiping gives
/// clock-free per-run isolation (no stale artifacts from a prior run). Returns the
/// resolved path used for the readback scan; pass `rel` itself as the `dotnet` argv.
pub fn prepare_output_dir(working_dir: &str, rel: &str) -> std::io::Result<PathBuf> {
    let dir = resolve(working_dir, rel);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::MockHost;

    #[test]
    fn dotnet_builder_sets_program_and_cwd() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, "/wd".into(), "s".into());
        let _ = dotnet(&ctx).arg("build").run();
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].program, "dotnet");
        assert_eq!(cmds[0].cwd.as_deref(), Some("/wd"));
        assert_eq!(cmds[0].args, vec!["build".to_string()]);
    }

    #[test]
    fn exit_phrase_formats_code() {
        assert_eq!(exit_phrase(2), "Dotnet command failed with exit code 2");
    }

    #[test]
    fn project_slug_distinguishes_same_stem_across_dirs() {
        assert_eq!(project_slug("Lib.csproj"), "Lib");
        assert_eq!(project_slug("a/Lib.csproj"), "a_Lib");
        assert_eq!(project_slug("b/Lib.csproj"), "b_Lib");
        assert_eq!(project_slug("src/MyApp/MyApp.csproj"), "src_MyApp_MyApp");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resolve_joins_working_dir_on_native() {
        assert_eq!(resolve("/wd", "a.csproj"), PathBuf::from("/wd/a.csproj"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn prepare_output_dir_wipes_existing_contents() {
        let d = tempfile::tempdir().unwrap();
        let wd = d.path().to_str().unwrap();
        let first = prepare_output_dir(wd, ".moonlit/dotnet/App").unwrap();
        std::fs::write(first.join("stale.nupkg"), b"x").unwrap();
        let second = prepare_output_dir(wd, ".moonlit/dotnet/App").unwrap();
        assert_eq!(first, second);
        assert!(second.is_dir());
        assert!(
            !second.join("stale.nupkg").exists(),
            "prepare must wipe prior contents"
        );
    }
}
