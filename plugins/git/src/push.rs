//! `git push` — push the current branch and (optionally) tags to a remote.

use crate::shared::{ensure_repo, git};
use moonlit_sdk::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PushConfig {
    remote: String,
    push_tags: bool,
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            push_tags: true,
        }
    }
}

#[derive(Default)]
pub struct Push;

impl Middleware for Push {
    const NAME: &'static str = "push";
    const DESCRIPTION: &'static str = "push the current branch and tags to a remote";
    type Config = PushConfig;

    fn execute(&self, ctx: &Context, cfg: PushConfig) -> MiddlewareResult {
        if let Err(f) = ensure_repo(ctx) {
            return f;
        }
        // `--end-of-options` keeps a `-`-leading remote name out of git's flag parser.
        match git(ctx)
            .arg("remote")
            .arg("get-url")
            .arg("--end-of-options")
            .arg(&cfg.remote)
            .run()
        {
            Ok(o) if o.success() => {}
            _ => return MiddlewareResult::failure(format!("Remote '{}' not found.", cfg.remote)),
        }

        let has_upstream = matches!(
            git(ctx)
                .arg("rev-parse")
                .arg("--abbrev-ref")
                .arg("--symbolic-full-name")
                .arg("@{u}")
                .run(),
            Ok(o) if o.success()
        );

        let mut result = MiddlewareResult::success();
        if !has_upstream {
            result = result.with_warning("current branch has no upstream configured");
        }

        // `--end-of-options` precedes the remote positional so a `-`-leading remote
        // cannot be read as a flag. For the tags push, `--tags` is our own option and
        // must stay *before* the marker; the remote follows it.
        if let Err(f) = run_push(ctx, &["push", "--end-of-options", &cfg.remote, "HEAD"]) {
            return f;
        }
        if cfg.push_tags {
            if let Err(f) = run_push(ctx, &["push", "--tags", "--end-of-options", &cfg.remote]) {
                return f;
            }
        }
        result
    }
}

/// Run one `git push …`; classify auth failures into a helpful hint.
fn run_push(ctx: &Context, args: &[&str]) -> Result<(), MiddlewareResult> {
    match git(ctx).args(args.iter().copied()).run() {
        Ok(o) if o.success() => Ok(()),
        Ok(o) => {
            let err = o.stderr();
            if err.contains("Permission denied (publickey)")
                || err.contains("Authentication failed")
                || err.contains("could not read Username")
            {
                Err(MiddlewareResult::failure(
                    "Git authentication failed. Ensure your SSH agent is running or your HTTPS credentials are configured.",
                ))
            } else {
                Err(MiddlewareResult::failure(format!(
                    "Git command failed with exit code {}",
                    o.exit_code
                )))
            }
        }
        Err(e) => Err(MiddlewareResult::failure(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::{run, MockHost};

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }
    fn err(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stderr,
            text: text.to_string(),
        }
    }

    #[test]
    fn pushes_branch_and_tags_by_default() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![out("git@github.com:me/repo.git")]) // remote get-url
            .with_process_result(0, vec![out("origin/main")]) // upstream probe
            .with_process_result(0, vec![]) // push HEAD
            .with_process_result(0, vec![]); // push --tags
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let result = run(&Push, &ctx, PushConfig::default());
        assert!(result.is_success());
        assert!(result.warnings().is_empty());
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[3].args,
            vec!["push", "--end-of-options", "origin", "HEAD"]
        );
        assert_eq!(
            cmds[4].args,
            vec!["push", "--tags", "--end-of-options", "origin"]
        );
    }

    #[test]
    fn missing_remote_fails() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(2, vec![]); // remote get-url fails
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = PushConfig {
            remote: "upstream".to_string(),
            ..Default::default()
        };
        let w = run(&Push, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Remote 'upstream' not found.")
        );
    }

    #[test]
    fn no_upstream_warns_but_succeeds() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![out("git@github.com:me/repo.git")])
            .with_process_result(128, vec![]) // upstream probe fails
            .with_process_result(0, vec![]) // push HEAD
            .with_process_result(0, vec![]); // push --tags
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let result = run(&Push, &ctx, PushConfig::default());
        assert!(result.is_success());
        assert_eq!(
            result.warnings(),
            &["current branch has no upstream configured".to_string()]
        );
    }

    #[test]
    fn skips_tags_when_disabled() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![out("git@github.com:me/repo.git")])
            .with_process_result(0, vec![out("origin/main")])
            .with_process_result(0, vec![]); // push HEAD only
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = PushConfig {
            push_tags: false,
            ..Default::default()
        };
        assert!(run(&Push, &ctx, cfg).is_success());
        assert_eq!(host.recorded_commands().len(), 4);
    }

    #[test]
    fn auth_failure_maps_to_hint() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![out("git@github.com:me/repo.git")])
            .with_process_result(0, vec![out("origin/main")])
            .with_process_result(
                128,
                vec![err("git@github.com: Permission denied (publickey).")],
            );
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let w = run(&Push, &ctx, PushConfig::default()).into_wit();
        assert!(!w.successful);
        assert!(w
            .error_message
            .as_deref()
            .unwrap()
            .contains("Git authentication failed"));
    }
}
