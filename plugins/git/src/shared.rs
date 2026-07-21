//! Shared state, the cwd-seeded `git` command builder, and the repo probe.

use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::Command;

/// Plugin-wide shared state (one instance per pipeline run).
#[derive(Default)]
pub struct GitShared {
    /// Commit SHA of the tag `latest-tag` matched; read by `commits`.
    // `latest-tag`/`commits` land in a later task; until then no middleware in
    // this crate reads the field, so silence the premature dead-code lint.
    #[allow(dead_code)]
    pub latest_tag_sha: Shared<Option<String>>,
}

/// Config type for middlewares that take no parameters.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct NoConfig {}

/// A `git` command pre-seeded with the repo working directory as cwd.
pub fn git<'a>(ctx: &Context<'a>) -> Command<'a> {
    ctx.command("git").cwd(ctx.working_dir())
}

/// Probe repo presence; map any failure to the canonical message.
pub fn ensure_repo(ctx: &Context) -> Result<(), MiddlewareResult> {
    match git(ctx).arg("rev-parse").arg("--git-dir").run() {
        Ok(out) if out.success() => Ok(()),
        _ => Err(MiddlewareResult::failure(
            "Not a git repository (or any of the parent directories)",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::MockHost;

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    #[test]
    fn ensure_repo_ok_when_git_dir_resolves() {
        let host = MockHost::new().with_process_result(0, vec![out(".git")]);
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        assert!(ensure_repo(&ctx).is_ok());
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].program, "git");
        assert_eq!(cmds[0].cwd.as_deref(), Some("/repo"));
        assert_eq!(cmds[0].args, vec!["rev-parse", "--git-dir"]);
    }

    #[test]
    fn ensure_repo_maps_failure_to_canonical_message() {
        let host = MockHost::new().with_process_result(128, vec![]);
        let ctx = Context::new(&host, "/nope".into(), "s".into());
        let msg = match ensure_repo(&ctx) {
            Ok(()) => panic!("expected a not-a-repo failure"),
            Err(f) => f.error_message().unwrap().to_string(),
        };
        assert_eq!(
            msg,
            "Not a git repository (or any of the parent directories)"
        );
    }
}
