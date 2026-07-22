//! `analyze` — parse raw commits into conventional commits, apply scope filters,
//! store them in shared state, and emit them (1.x `ConvertCommits`).

use moonlit_plugin_sdk::prelude::*;

use crate::convert::convert_all;
use crate::models::{Commit, ConventionalCommit, SrShared};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AnalyzeConfig {
    commits: Vec<Commit>,
    include_scopes: Option<Vec<String>>,
    exclude_scopes: Option<Vec<String>>,
    include_unscoped: bool,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            commits: Vec::new(),
            include_scopes: None,
            exclude_scopes: None,
            include_unscoped: true,
        }
    }
}

#[derive(Default)]
pub struct Analyze;

impl Middleware for Analyze {
    const NAME: &'static str = "analyze";
    const DESCRIPTION: &'static str = "parse raw commits into conventional commits";
    type Config = AnalyzeConfig;

    fn execute(&self, ctx: &Context, cfg: AnalyzeConfig) -> MiddlewareResult {
        let filtered: Vec<ConventionalCommit> = convert_all(&cfg.commits)
            .into_iter()
            .filter(|c| keep(c, &cfg))
            .collect();
        ctx.state::<SrShared>().commits.set(filtered.clone());
        let count = filtered.len() as i64;
        MiddlewareResult::success_with(move |o| {
            o.set("commits", filtered);
            o.set("commitCount", count);
        })
    }
}

/// 1.x `CheckCommit`: unscoped -> includeUnscoped; else includeScopes (if any) wins,
/// then excludeScopes (if any), else keep. Scope comparison is case-sensitive (1.x).
fn keep(c: &ConventionalCommit, cfg: &AnalyzeConfig) -> bool {
    match &c.scope {
        None => cfg.include_unscoped,
        Some(scope) => {
            if let Some(inc) = cfg.include_scopes.as_ref().filter(|v| !v.is_empty()) {
                return inc.iter().any(|s| s == scope);
            }
            if let Some(exc) = cfg.exclude_scopes.as_ref().filter(|v| !v.is_empty()) {
                return !exc.iter().any(|s| s == scope);
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};
    use serde_json::Value;

    fn cfg(json: Value) -> AnalyzeConfig {
        moonlit_plugin_sdk::config::from_json_value(&json.to_string()).unwrap()
    }

    fn outputs(w: moonlit_plugin_sdk::bindings::MiddlewareResult) -> std::collections::HashMap<String, Value> {
        w.output.into_iter().map(|(k, v)| (k, serde_json::from_str(&v).unwrap())).collect()
    }

    #[test]
    fn converts_and_counts_and_writes_shared() {
        let shared = SrShared::default();
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_state(&shared);
        let c = cfg(serde_json::json!({
            "commits": [
                { "sha": "aaaaaaa1", "date": "2026-01-01T00:00:00Z", "message": "feat: a" },
                { "sha": "bbbbbbb1", "date": "2026-01-02T00:00:00Z", "message": "chore: b" }
            ]
        }));
        let w = run(&Analyze, &ctx, c).into_wit();
        assert!(w.successful);
        let out = outputs(w);
        assert_eq!(out["commitCount"], serde_json::json!(2));
        assert_eq!(out["commits"].as_array().unwrap().len(), 2);
        assert_eq!(out["commits"][0]["type"], "feat");
        // shared state now holds the two parsed commits
        assert_eq!(shared.commits.get().len(), 2);
    }

    #[test]
    fn drops_unscoped_when_include_unscoped_false() {
        let shared = SrShared::default();
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_state(&shared);
        let c = cfg(serde_json::json!({
            "includeUnscoped": false,
            "commits": [
                { "sha": "a1", "date": "2026-01-01T00:00:00Z", "message": "feat: no scope" },
                { "sha": "b1", "date": "2026-01-01T00:00:00Z", "message": "feat(cli): scoped" }
            ]
        }));
        let w = run(&Analyze, &ctx, c).into_wit();
        let out = outputs(w);
        assert_eq!(out["commitCount"], serde_json::json!(1));
        assert_eq!(out["commits"][0]["scope"], "cli");
    }

    #[test]
    fn include_scopes_takes_precedence_over_exclude_scopes() {
        let shared = SrShared::default();
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_state(&shared);
        let c = cfg(serde_json::json!({
            "includeScopes": ["cli"],
            "excludeScopes": ["cli"],
            "commits": [
                { "sha": "a1", "date": "2026-01-01T00:00:00Z", "message": "feat(cli): kept" },
                { "sha": "b1", "date": "2026-01-01T00:00:00Z", "message": "feat(api): dropped" }
            ]
        }));
        let w = run(&Analyze, &ctx, c).into_wit();
        let out = outputs(w);
        assert_eq!(out["commitCount"], serde_json::json!(1));
        assert_eq!(out["commits"][0]["scope"], "cli");
    }

    #[test]
    fn empty_commits_succeed_with_zero_count() {
        let shared = SrShared::default();
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_state(&shared);
        let w = run(&Analyze, &ctx, AnalyzeConfig::default()).into_wit();
        assert!(w.successful);
        let out = outputs(w);
        assert_eq!(out["commitCount"], serde_json::json!(0));
    }
}
