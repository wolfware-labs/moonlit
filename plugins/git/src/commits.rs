//! `git commits` — commits in a range, newest-first, boundary excluded.

use crate::shared::{ensure_repo, git, GitShared};
use moonlit_plugin_sdk::prelude::*;

/// `git log` format: field sep 0x1f, record sep 0x1e. `%B` (raw body) is last so
/// its internal newlines never collide with the field separators.
const LOG_FORMAT: &str = "--format=%H%x1f%an%x1f%ae%x1f%aI%x1f%B%x1e";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CommitsConfig {
    since_sha: Option<String>,
    use_shared_context: bool,
    since: Option<String>,
    until: String,
}

impl Default for CommitsConfig {
    fn default() -> Self {
        Self {
            since_sha: None,
            use_shared_context: true,
            since: None,
            until: "HEAD".to_string(),
        }
    }
}

#[derive(serde::Serialize)]
struct Commit {
    sha: String,
    author: String,
    email: String,
    date: String,
    message: String,
}

fn parse_commits(raw: &str) -> Vec<Commit> {
    raw.split('\u{1e}')
        .map(str::trim)
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let mut f = record.splitn(5, '\u{1f}');
            Some(Commit {
                sha: f.next()?.to_string(),
                author: f.next()?.to_string(),
                email: f.next()?.to_string(),
                date: f.next()?.to_string(),
                message: f.next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

#[derive(Default)]
pub struct Commits;

impl Middleware for Commits {
    const NAME: &'static str = "commits";
    const DESCRIPTION: &'static str = "commits in a range (newest-first, boundary excluded)";
    type Config = CommitsConfig;

    fn execute(&self, ctx: &Context, cfg: CommitsConfig) -> MiddlewareResult {
        if let Err(f) = ensure_repo(ctx) {
            return f;
        }

        let boundary: Option<String> = if let Some(s) = cfg.since_sha.filter(|s| !s.is_empty()) {
            Some(s)
        } else if let Some(since) = cfg.since.filter(|s| !s.is_empty()) {
            match git(ctx).arg("rev-parse").arg(&since).run() {
                Ok(o) if o.success() => Some(o.stdout().trim().to_string()),
                _ => {
                    return MiddlewareResult::failure(format!(
                        "Could not resolve 'since' reference: {since}"
                    ))
                }
            }
        } else if cfg.use_shared_context {
            ctx.state::<GitShared>().latest_tag_sha.get()
        } else {
            None
        };

        let range = match &boundary {
            Some(b) => format!("{b}..{}", cfg.until),
            None => cfg.until.clone(),
        };

        let out = match git(ctx).arg("log").arg(&range).arg(LOG_FORMAT).run() {
            Ok(o) if o.success() => o,
            Ok(o) => {
                return MiddlewareResult::failure(format!(
                    "Git command failed with exit code {}",
                    o.exit_code
                ))
            }
            Err(e) => return MiddlewareResult::failure(e),
        };

        let details = parse_commits(&out.stdout());
        let count = details.len() as i64;
        MiddlewareResult::success_with(|o| {
            o.set("details", &details);
            o.set("count", count);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::GitShared;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    // A whole git-log payload as one chunk: `Output::stdout()` returns it verbatim
    // (MockHost does not re-split on newlines), so embedded \n / \x1f / \x1e survive.
    fn log_payload() -> &'static str {
        "aaa\u{1f}Ada\u{1f}ada@x\u{1f}2026-01-02T00:00:00Z\u{1f}fix: two\nbody line\u{1e}\nbbb\u{1f}Bob\u{1f}bob@x\u{1f}2026-01-01T00:00:00Z\u{1f}feat: one\u{1e}"
    }

    #[test]
    fn parses_multiline_messages_and_counts() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![out(log_payload())]); // git log
        let shared = GitShared::default();
        // useSharedContext defaults true but state is empty -> boundary None -> HEAD.
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let w = run(&Commits, &ctx, CommitsConfig::default()).into_wit();
        assert!(w.successful);
        let map: std::collections::HashMap<String, serde_json::Value> = w
            .output
            .into_iter()
            .map(|(k, v)| (k, serde_json::from_str(&v).unwrap()))
            .collect();
        assert_eq!(map["count"], serde_json::json!(2));
        let details = map["details"].as_array().unwrap();
        assert_eq!(details.len(), 2);
        assert_eq!(details[0]["sha"], "aaa");
        assert_eq!(details[0]["message"], "fix: two\nbody line");
        assert_eq!(details[0]["date"], "2026-01-02T00:00:00Z");
        // range was plain HEAD (no boundary)
        let cmds = host.recorded_commands();
        assert_eq!(cmds[1].args, vec!["log", "HEAD", LOG_FORMAT]);
    }

    #[test]
    fn shared_context_boundary_excludes_tag_commit() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(
                0,
                vec![out(
                    "aaa\u{1f}Ada\u{1f}ada@x\u{1f}2026-01-02T00:00:00Z\u{1f}fix\u{1e}",
                )],
            );
        let shared = GitShared::default();
        shared.latest_tag_sha.set(Some("tagsha".to_string()));
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let w = run(&Commits, &ctx, CommitsConfig::default()).into_wit();
        assert!(w.successful);
        let cmds = host.recorded_commands();
        assert_eq!(cmds[1].args, vec!["log", "tagsha..HEAD", LOG_FORMAT]);
    }

    #[test]
    fn since_sha_takes_precedence_over_shared() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![out("")]); // empty range
        let shared = GitShared::default();
        shared.latest_tag_sha.set(Some("ignored".to_string()));
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let cfg = CommitsConfig {
            since_sha: Some("explicit".to_string()),
            ..Default::default()
        };
        let w = run(&Commits, &ctx, cfg).into_wit();
        assert!(w.successful);
        let map: std::collections::HashMap<String, serde_json::Value> = w
            .output
            .into_iter()
            .map(|(k, v)| (k, serde_json::from_str(&v).unwrap()))
            .collect();
        assert_eq!(map["count"], serde_json::json!(0));
        let cmds = host.recorded_commands();
        assert_eq!(cmds[1].args, vec!["log", "explicit..HEAD", LOG_FORMAT]);
    }

    #[test]
    fn unresolvable_since_fails() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(128, vec![]); // rev-parse <since> fails
        let shared = GitShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let cfg = CommitsConfig {
            since: Some("nope".to_string()),
            ..Default::default()
        };
        let w = run(&Commits, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Could not resolve 'since' reference: nope")
        );
    }

    #[test]
    fn since_ref_is_resolved_then_used_as_boundary() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![out("resolvedsha")]) // rev-parse main
            .with_process_result(0, vec![out("")]); // git log
        let shared = GitShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let cfg = CommitsConfig {
            since: Some("main".to_string()),
            ..Default::default()
        };
        let w = run(&Commits, &ctx, cfg).into_wit();
        assert!(w.successful);
        let cmds = host.recorded_commands();
        assert_eq!(cmds[1].args, vec!["rev-parse", "main"]);
        assert_eq!(cmds[2].args, vec!["log", "resolvedsha..HEAD", LOG_FORMAT]);
    }
}
