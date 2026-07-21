//! `git repo-context` — current branch and origin remote URL.

use crate::shared::{ensure_repo, git, NoConfig};
use moonlit_plugin_sdk::prelude::*;

#[derive(Default)]
pub struct RepoContext;

impl Middleware for RepoContext {
    const NAME: &'static str = "repo-context";
    const DESCRIPTION: &'static str = "current branch and origin remote URL";
    type Config = NoConfig;

    fn execute(&self, ctx: &Context, _cfg: NoConfig) -> MiddlewareResult {
        if let Err(f) = ensure_repo(ctx) {
            return f;
        }
        let branch = match git(ctx)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .run()
        {
            Ok(o) if o.success() => o.stdout().trim().to_string(),
            Ok(o) => {
                return MiddlewareResult::failure(format!(
                    "Git command failed with exit code {}",
                    o.exit_code
                ))
            }
            Err(e) => return MiddlewareResult::failure(e),
        };
        let remote_url = match git(ctx).arg("remote").arg("get-url").arg("origin").run() {
            Ok(o) if o.success() => o.stdout().trim().to_string(),
            _ => return MiddlewareResult::failure("Remote 'origin' not found."),
        };
        MiddlewareResult::success_with(|o| {
            o.set("branch", branch);
            o.set("remoteUrl", remote_url);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    #[test]
    fn emits_branch_and_remote_url() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![out("main")]) // branch
            .with_process_result(0, vec![out("git@github.com:me/repo.git")]); // remote
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let w = run(&RepoContext, &ctx, NoConfig::default()).into_wit();
        assert!(w.successful);
        let map: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(map["branch"], "\"main\"");
        assert_eq!(map["remoteUrl"], "\"git@github.com:me/repo.git\"");
        let cmds = host.recorded_commands();
        assert!(cmds.iter().all(|c| c.cwd.as_deref() == Some("/repo")));
        assert_eq!(cmds[1].args, vec!["rev-parse", "--abbrev-ref", "HEAD"]);
        assert_eq!(cmds[2].args, vec!["remote", "get-url", "origin"]);
    }

    #[test]
    fn missing_origin_fails_with_exact_message() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![out("main")])
            .with_process_result(128, vec![]); // remote get-url fails
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let w = run(&RepoContext, &ctx, NoConfig::default()).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Remote 'origin' not found.")
        );
    }

    #[test]
    fn not_a_repo_fails_with_canonical_message() {
        let host = MockHost::new().with_process_result(128, vec![]);
        let ctx = Context::new(&host, "/nope".into(), "s".into());
        let w = run(&RepoContext, &ctx, NoConfig::default()).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Not a git repository (or any of the parent directories)")
        );
    }
}
