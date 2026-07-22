//! Shared `dotnet` command helpers: cwd-seeded builder, project-path resolution
//! against the wasm preopen, and the uniform non-zero-exit phrase.

use moonlit_plugin_sdk::process::Command;
use moonlit_plugin_sdk::Context;
use std::path::PathBuf;

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

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::MockHost;

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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resolve_joins_working_dir_on_native() {
        assert_eq!(resolve("/wd", "a.csproj"), PathBuf::from("/wd/a.csproj"));
    }
}
