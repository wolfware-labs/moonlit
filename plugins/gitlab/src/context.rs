//! GitLab project path, derived once per run from the `origin` remote and cached.

use moonlit_plugin_sdk::prelude::*;
use regex::Regex;

use crate::config::GitlabPluginConfig;

#[derive(Clone)]
pub struct GitlabContext {
    pub project_path: String, // "group/subgroup/project" (human — used in web/commit URLs)
    pub project_id: String,   // "group%2Fsubgroup%2Fproject" (URL-encoded — API :id)
    pub base_url: String,     // "https://gitlab.com" (no trailing slash)
}

impl GitlabContext {
    pub fn web_url(&self) -> String {
        format!("{}/{}", self.base_url, self.project_path)
    }
    pub fn commit_url_prefix(&self) -> String {
        format!("{}/{}/-/commit/", self.base_url, self.project_path)
    }
    pub fn api_base(&self) -> String {
        format!("{}/api/v4", self.base_url)
    }
}

/// Plugin-wide shared state (one instance per pipeline run).
#[derive(Default)]
pub struct GitlabShared {
    pub context: Shared<Option<GitlabContext>>,
}

/// Percent-encode a project path for use as the API `:id`: `/` becomes `%2F` and
/// any byte outside the unreserved set `[A-Za-z0-9._-]` is percent-encoded.
fn encode_project_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 8);
    for &b in path.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Host part of a base URL like `https://gitlab.example.com` -> `gitlab.example.com`.
fn host_of(base_url: &str) -> &str {
    let after_scheme = base_url.split_once("://").map(|(_, r)| r).unwrap_or(base_url);
    after_scheme.split('/').next().unwrap_or(after_scheme)
}

/// Resolve the GitLab project path, caching the result for the run. Reads
/// `base_url` from plugin config (github's context needs none), shells `git remote
/// get-url origin` (no user-controlled positional → no injection surface), and
/// parses the project path out of the remote URL.
pub fn resolve_context(ctx: &Context) -> Result<GitlabContext, MiddlewareResult> {
    if let Some(c) = ctx.state::<GitlabShared>().context.get() {
        return Ok(c);
    }
    let base_url = {
        let raw = ctx.plugin_config::<GitlabPluginConfig>().base_url.clone();
        let trimmed = raw.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            "https://gitlab.com".to_string()
        } else {
            trimmed.to_string()
        }
    };
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
    let host = host_of(&base_url);
    let pattern = format!(r"{}[/:](?P<path>.+?)(?:\.git)?$", regex::escape(host));
    let re = Regex::new(&pattern).unwrap();
    let caps = match re.captures(&url) {
        Some(c) => c,
        None => return Err(MiddlewareResult::failure("Not a valid GitLab URL.")),
    };
    let project_path = caps["path"].trim_end_matches('/').to_string();
    let context = GitlabContext {
        project_id: encode_project_path(&project_path),
        project_path,
        base_url,
    };
    ctx.state::<GitlabShared>().context.set(Some(context.clone()));
    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GitlabPluginConfig;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::MockHost;

    fn out(text: &str) -> OutputChunk {
        OutputChunk { stream: StdioStream::Stdout, text: text.to_string() }
    }
    fn pc(base_url: &str) -> GitlabPluginConfig {
        GitlabPluginConfig { token: "t".into(), base_url: base_url.into() }
    }
    fn ctx_with<'a>(host: &'a MockHost, sh: &'a GitlabShared, cfg: &'a GitlabPluginConfig) -> Context<'a> {
        Context::new(host, "/repo".into(), "s".into()).with_state(sh).with_plugin_config(cfg)
    }

    #[test]
    fn encode_project_path_encodes_slashes_and_specials() {
        assert_eq!(encode_project_path("group/sub/project"), "group%2Fsub%2Fproject");
        assert_eq!(encode_project_path("a b/c"), "a%20b%2Fc");
    }

    #[test]
    fn parses_https_nested_group_url() {
        let host = MockHost::new().with_process_result(0, vec![out("https://gitlab.com/group/subgroup/project.git")]);
        let sh = GitlabShared::default();
        let cfg = pc("https://gitlab.com");
        let ctx = ctx_with(&host, &sh, &cfg);
        let c = resolve_context(&ctx).unwrap_or_else(|_| panic!("must resolve"));
        assert_eq!(c.project_path, "group/subgroup/project");
        assert_eq!(c.project_id, "group%2Fsubgroup%2Fproject");
        assert_eq!(c.web_url(), "https://gitlab.com/group/subgroup/project");
        assert_eq!(c.commit_url_prefix(), "https://gitlab.com/group/subgroup/project/-/commit/");
        assert_eq!(c.api_base(), "https://gitlab.com/api/v4");
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].program, "git");
        assert_eq!(cmds[0].cwd.as_deref(), Some("/repo"));
        assert_eq!(cmds[0].args, vec!["remote", "get-url", "origin"]);
    }

    #[test]
    fn parses_ssh_url() {
        let host = MockHost::new().with_process_result(0, vec![out("git@gitlab.com:me/repo.git")]);
        let sh = GitlabShared::default();
        let cfg = pc("https://gitlab.com");
        let ctx = ctx_with(&host, &sh, &cfg);
        let c = resolve_context(&ctx).unwrap_or_else(|_| panic!("must resolve"));
        assert_eq!(c.project_path, "me/repo");
        assert_eq!(c.project_id, "me%2Frepo");
    }

    #[test]
    fn custom_base_url_matches_self_hosted_host() {
        let host = MockHost::new().with_process_result(0, vec![out("https://gitlab.example.com/team/app.git")]);
        let sh = GitlabShared::default();
        let cfg = pc("https://gitlab.example.com/");
        let ctx = ctx_with(&host, &sh, &cfg);
        let c = resolve_context(&ctx).unwrap_or_else(|_| panic!("must resolve"));
        assert_eq!(c.project_path, "team/app");
        assert_eq!(c.api_base(), "https://gitlab.example.com/api/v4");
    }

    #[test]
    fn non_gitlab_url_fails_with_exact_message() {
        let host = MockHost::new().with_process_result(0, vec![out("https://github.com/me/repo.git")]);
        let sh = GitlabShared::default();
        let cfg = pc("https://gitlab.com");
        let ctx = ctx_with(&host, &sh, &cfg);
        let msg = match resolve_context(&ctx) {
            Ok(_) => panic!("github url must fail"),
            Err(f) => f.error_message().unwrap().to_string(),
        };
        assert_eq!(msg, "Not a valid GitLab URL.");
    }

    #[test]
    fn missing_origin_fails_with_exact_message() {
        let host = MockHost::new().with_process_result(128, vec![]);
        let sh = GitlabShared::default();
        let cfg = pc("https://gitlab.com");
        let ctx = ctx_with(&host, &sh, &cfg);
        let msg = match resolve_context(&ctx) {
            Ok(_) => panic!("missing origin must fail"),
            Err(f) => f.error_message().unwrap().to_string(),
        };
        assert_eq!(msg, "Remote 'origin' not found.");
    }

    #[test]
    fn result_is_cached_git_runs_once() {
        // Only ONE process result enqueued: a second git call would hit the MockHost
        // "no process result configured" error, so a cache miss fails.
        let host = MockHost::new().with_process_result(0, vec![out("https://gitlab.com/o/r.git")]);
        let sh = GitlabShared::default();
        let cfg = pc("https://gitlab.com");
        let ctx = ctx_with(&host, &sh, &cfg);
        let a = resolve_context(&ctx).unwrap_or_else(|_| panic!("first resolve"));
        let b = resolve_context(&ctx).unwrap_or_else(|_| panic!("cached resolve"));
        assert_eq!(a.project_id, b.project_id);
        assert_eq!(host.recorded_commands().len(), 1, "git must run exactly once");
    }
}
