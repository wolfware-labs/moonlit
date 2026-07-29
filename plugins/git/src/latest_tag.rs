//! `git latest-tag` — newest tag matching a pattern; stores its commit SHA.

use crate::shared::{ensure_repo, git, GitShared};
use moonlit_sdk::prelude::*;
use regex::RegexBuilder;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LatestTagConfig {
    prefix: String,
    suffix: String,
    pattern: String,
}

impl Default for LatestTagConfig {
    fn default() -> Self {
        // Manual Default so a missing `pattern` takes the 1.x default rather than
        // the empty string `#[derive(Default)]` would give.
        Self {
            prefix: String::new(),
            suffix: String::new(),
            pattern: "[0-9]+.[0-9]+.[0-9]+.*".to_string(),
        }
    }
}

#[derive(Default)]
pub struct LatestTag;

impl Middleware for LatestTag {
    const NAME: &'static str = "latest-tag";
    const DESCRIPTION: &'static str = "newest tag matching a pattern (stores its commit SHA)";
    type Config = LatestTagConfig;

    fn execute(&self, ctx: &Context, cfg: LatestTagConfig) -> MiddlewareResult {
        if let Err(f) = ensure_repo(ctx) {
            return f;
        }
        let re = match RegexBuilder::new(&format!("^{}{}{}$", cfg.prefix, cfg.pattern, cfg.suffix))
            .case_insensitive(true)
            .build()
        {
            Ok(re) => re,
            Err(_) => {
                return MiddlewareResult::failure(format!("Invalid tag pattern: {}", cfg.pattern))
            }
        };
        let listing = match git(ctx)
            .arg("for-each-ref")
            .arg("--sort=-creatordate")
            .arg("--format=%(refname:short) %(objectname) %(*objectname)")
            .arg("refs/tags")
            .run()
        {
            Ok(o) if o.success() => o,
            Ok(o) => {
                return MiddlewareResult::failure(format!(
                    "Git command failed with exit code {}",
                    o.exit_code
                ))
            }
            Err(e) => return MiddlewareResult::failure(e),
        };

        for line in listing.stdout().lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split(' ');
            let refname = parts.next().unwrap_or("");
            let objectname = parts.next().unwrap_or("");
            let star = parts.next().unwrap_or("");
            if !re.is_match(refname) {
                continue;
            }
            let commit_sha = if star.is_empty() { objectname } else { star }.to_string();
            let stripped = refname.strip_prefix(&cfg.prefix).unwrap_or(refname);
            let name = stripped
                .strip_suffix(&cfg.suffix)
                .unwrap_or(stripped)
                .to_string();
            let full_name = refname.to_string();

            ctx.state::<GitShared>()
                .latest_tag_sha
                .set(Some(commit_sha.clone()));

            return MiddlewareResult::success_with(|o| {
                o.set("name", name);
                o.set("fullName", full_name);
                o.set("commitSha", commit_sha);
            });
        }

        MiddlewareResult::success().with_warning("No matching tags found.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::GitShared;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::{run, MockHost};

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    #[test]
    fn matches_prefixed_tag_strips_prefix_and_stores_sha() {
        // Lightweight tag has an empty *objectname (trailing space).
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(
                0,
                vec![out("v2.1.0 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ")],
            );
        let shared = GitShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let cfg = LatestTagConfig {
            prefix: "v".to_string(),
            ..Default::default()
        };
        let w = run(&LatestTag, &ctx, cfg).into_wit();
        assert!(w.successful);
        let map: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(map["name"], "\"2.1.0\"");
        assert_eq!(map["fullName"], "\"v2.1.0\"");
        assert_eq!(
            map["commitSha"],
            "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""
        );
        assert_eq!(
            shared.latest_tag_sha.get(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
    }

    #[test]
    fn annotated_tag_uses_peeled_commit() {
        // Annotated tag: objectname is the tag object, *objectname the commit.
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(
                0,
                vec![out(
                    "1.0.0 tttttttttttttttttttttttttttttttttttttttt cccccccccccccccccccccccccccccccccccccccc",
                )],
            );
        let shared = GitShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let w = run(&LatestTag, &ctx, LatestTagConfig::default()).into_wit();
        assert!(w.successful);
        let map: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(
            map["commitSha"],
            "\"cccccccccccccccccccccccccccccccccccccccc\""
        );
    }

    #[test]
    fn no_match_warns_and_succeeds_without_outputs() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![out("release-candidate zzzz ")]);
        let shared = GitShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let result = run(&LatestTag, &ctx, LatestTagConfig::default());
        assert!(result.is_success());
        assert_eq!(result.warnings(), &["No matching tags found.".to_string()]);
        assert!(result.into_wit().output.is_empty());
        assert_eq!(shared.latest_tag_sha.get(), None);
    }

    #[test]
    fn bad_pattern_fails() {
        let host = MockHost::new().with_process_result(0, vec![out(".git")]);
        let shared = GitShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let cfg = LatestTagConfig {
            pattern: "[".to_string(),
            ..Default::default()
        };
        let w = run(&LatestTag, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert!(w
            .error_message
            .as_deref()
            .unwrap()
            .starts_with("Invalid tag pattern:"));
    }
}
