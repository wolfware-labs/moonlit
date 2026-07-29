//! GitHub owner/repo, derived once per run from the `origin` remote and cached.

use moonlit_sdk::prelude::*;
use regex::Regex;

#[derive(Clone)]
pub struct GithubContext {
    pub owner: String,
    pub repo: String,
}

impl GithubContext {
    pub fn commit_url_prefix(&self) -> String {
        format!("https://github.com/{}/{}/commit/", self.owner, self.repo)
    }
}

/// Plugin-wide shared state (one instance per pipeline run).
#[derive(Default)]
pub struct GithubShared {
    pub context: Shared<Option<GithubContext>>,
}

/// Resolve owner/repo, caching the result for the run. Shells `git remote
/// get-url origin` (no user-controlled positional → no injection surface) and
/// parses the GitHub URL.
pub fn resolve_context(ctx: &Context) -> Result<GithubContext, MiddlewareResult> {
    if let Some(c) = ctx.state::<GithubShared>().context.get() {
        return Ok(c);
    }
    let url = match ctx
        .command("git")
        .cwd(ctx.working_dir())
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .run()
    {
        Ok(o) if o.success() => o.stdout().trim().to_string(),
        _ => return Err(MiddlewareResult::failure("Remote 'origin' not found.")),
    };
    // Anchor the host on a start/`/`/`@` boundary so a look-alike host such as
    // `evilgithub.com` (where the real host is only a substring) cannot match.
    let re = Regex::new(r"(?:^|[/@])github\.com[/:](?P<owner>[^/]+?)/(?P<repo>[^/.]+)(\.git)?$")
        .unwrap();
    let caps = match re.captures(&url) {
        Some(c) => c,
        None => return Err(MiddlewareResult::failure("Not a valid GitHub URL.")),
    };
    let context = GithubContext {
        owner: caps["owner"].to_string(),
        repo: caps["repo"].to_string(),
    };
    ctx.state::<GithubShared>()
        .context
        .set(Some(context.clone()));
    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::MockHost;

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    #[test]
    fn parses_https_url() {
        let host = MockHost::new()
            .with_process_result(0, vec![out("https://github.com/octo/Hello-World.git")]);
        let shared = GithubShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let c = resolve_context(&ctx).unwrap_or_else(|_| panic!("must resolve"));
        assert_eq!(c.owner, "octo");
        assert_eq!(c.repo, "Hello-World");
        assert_eq!(
            c.commit_url_prefix(),
            "https://github.com/octo/Hello-World/commit/"
        );
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].program, "git");
        assert_eq!(cmds[0].cwd.as_deref(), Some("/repo"));
        assert_eq!(cmds[0].args, vec!["remote", "get-url", "origin"]);
    }

    #[test]
    fn parses_ssh_url() {
        let host = MockHost::new().with_process_result(0, vec![out("git@github.com:me/repo.git")]);
        let shared = GithubShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let c = resolve_context(&ctx).unwrap_or_else(|_| panic!("must resolve"));
        assert_eq!(c.owner, "me");
        assert_eq!(c.repo, "repo");
    }

    #[test]
    fn non_github_url_fails_with_exact_message() {
        let host =
            MockHost::new().with_process_result(0, vec![out("https://gitlab.com/me/repo.git")]);
        let shared = GithubShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let msg = match resolve_context(&ctx) {
            Ok(_) => panic!("gitlab url must fail"),
            Err(f) => f.error_message().unwrap().to_string(),
        };
        assert_eq!(msg, "Not a valid GitHub URL.");
    }

    #[test]
    fn missing_origin_fails_with_exact_message() {
        let host = MockHost::new().with_process_result(128, vec![]);
        let shared = GithubShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let msg = match resolve_context(&ctx) {
            Ok(_) => panic!("missing origin must fail"),
            Err(f) => f.error_message().unwrap().to_string(),
        };
        assert_eq!(msg, "Remote 'origin' not found.");
    }

    #[test]
    fn lookalike_host_is_rejected() {
        // The real host appears only as a substring of `evilgithub.com`; the host
        // boundary must reject it rather than derive owner/repo from a foreign host.
        let host =
            MockHost::new().with_process_result(0, vec![out("https://evilgithub.com/me/repo.git")]);
        let shared = GithubShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let msg = match resolve_context(&ctx) {
            Ok(_) => panic!("look-alike host must fail"),
            Err(f) => f.error_message().unwrap().to_string(),
        };
        assert_eq!(msg, "Not a valid GitHub URL.");
    }

    #[test]
    fn result_is_cached_git_runs_once() {
        // Only ONE process result enqueued: a second git call would return the
        // MockHost "no process result configured" error, so a cache miss fails.
        let host = MockHost::new().with_process_result(0, vec![out("https://github.com/o/r.git")]);
        let shared = GithubShared::default();
        let ctx = Context::new(&host, "/repo".into(), "s".into()).with_state(&shared);
        let a = resolve_context(&ctx).unwrap_or_else(|_| panic!("first resolve"));
        let b = resolve_context(&ctx).unwrap_or_else(|_| panic!("cached resolve"));
        assert_eq!(a.owner, b.owner);
        assert_eq!(
            host.recorded_commands().len(),
            1,
            "git must run exactly once"
        );
    }
}
