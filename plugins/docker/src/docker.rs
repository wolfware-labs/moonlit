//! Shared helpers for the docker middlewares: the `docker` command builder and
//! the frozen non-zero-exit failure phrasing.

use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::Command;

/// A `docker` command rooted at the working dir. The spawn cwd is a HOST path —
/// the engine passes `cmd.cwd` verbatim to the OS process's `current_dir` — so
/// it is `ctx.working_dir()` directly (same seam lesson as nodejs's `npm()`).
pub fn docker<'a>(ctx: &Context<'a>) -> Command<'a> {
    ctx.command("docker").cwd(ctx.working_dir())
}

/// The frozen exit phrase for a non-zero `docker` exit (MVP_SPEC §11, line 653).
pub fn exit_phrase(code: i32) -> String {
    format!("Docker command failed with exit code {code}")
}

/// A branded failure for a non-zero exit: `"Failed to {action}: {exit_phrase}"`.
pub fn fail(action: &str, code: i32) -> MiddlewareResult {
    MiddlewareResult::failure(format!("Failed to {action}: {}", exit_phrase(code)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::MockHost;

    #[test]
    fn docker_sets_program_and_working_dir_cwd() {
        // Inspect the built command via a MockHost round-trip (the SDK `Command`
        // exposes no direct accessor — same approach as nodejs's npm() test).
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, "/wd".into(), "s".into());
        let _ = docker(&ctx).arg("version").run();
        let cmd = &host.recorded_commands()[0];
        assert_eq!(cmd.program, "docker");
        assert_eq!(cmd.cwd.as_deref(), Some("/wd"));
        assert_eq!(cmd.args, vec!["version".to_string()]);
    }

    #[test]
    fn exit_phrase_is_frozen() {
        assert_eq!(exit_phrase(2), "Docker command failed with exit code 2");
    }

    #[test]
    fn fail_wraps_action_and_exit_phrase() {
        let w = fail("build and push image", 1).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to build and push image: Docker command failed with exit code 1")
        );
    }
}
