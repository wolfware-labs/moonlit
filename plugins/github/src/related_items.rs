//! `github related-items` — merged PRs (and their linked issues) for a commit set.

use std::collections::HashSet;

use moonlit_plugin_sdk::prelude::*;
use serde::Serialize;
use serde_json::Value;

use crate::api;
use crate::config::GithubPluginConfig;
use crate::context::resolve_context;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitRef {
    sha: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RelatedItemsConfig {
    commits: Vec<CommitRef>,
    include_pull_requests: bool,
    include_issues: bool,
}

impl Default for RelatedItemsConfig {
    fn default() -> Self {
        Self {
            commits: Vec::new(),
            include_pull_requests: true,
            include_issues: true,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrOut {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IssueOut {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    pull_request_number: i64,
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
fn opt(v: &Value, k: &str) -> Option<String> {
    v[k].as_str().map(String::from)
}

fn map_pr(v: &Value) -> PrOut {
    PrOut {
        number: v["number"].as_i64().unwrap_or(0),
        title: s(v, "title"),
        body: opt(v, "body"),
        state: s(v, "state"),
        created_at: s(v, "created_at"),
        updated_at: s(v, "updated_at"),
        merged_at: opt(v, "merged_at"),
        merge_commit_sha: opt(v, "merge_commit_sha"),
    }
}

/// Issue numbers referenced by a PR body via `close/fix/resolve #N`. Case-insensitive.
///
/// Deliberate, verified divergence from 1.x: the C#/.NET plugin derives issues from
/// `Issue.GetAllForRepository` filtered by `issue.PullRequest.Number ∈ merged-PR set`.
/// That endpoint returns PRs-as-issues and does not populate `PullRequest.Number`, so
/// 1.x's issue linkage yields nothing in practice. Parsing the PR body for closing
/// keywords is the mechanism that actually produces linked issues, so we keep it.
fn referenced_issues(body: &str) -> Vec<i64> {
    let re =
        regex::Regex::new(r"(?i)\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+#(\d+)").unwrap();
    re.captures_iter(body)
        .filter_map(|c| c[1].parse::<i64>().ok())
        .collect()
}

#[derive(Default)]
pub struct RelatedItems;

impl Middleware for RelatedItems {
    const NAME: &'static str = "related-items";
    const DESCRIPTION: &'static str = "merged PRs and linked issues for a commit set";
    type Config = RelatedItemsConfig;

    fn execute(&self, ctx: &Context, cfg: RelatedItemsConfig) -> MiddlewareResult {
        if cfg.commits.is_empty() {
            ctx.log_info("No commits provided; skipping related-items lookup.");
            return MiddlewareResult::success();
        }
        let context = match resolve_context(ctx) {
            Ok(c) => c,
            Err(f) => return f,
        };
        let token = ctx.plugin_config::<GithubPluginConfig>().token.clone();
        let shas: HashSet<String> = cfg.commits.into_iter().map(|c| c.sha).collect();

        let mut prs: Vec<PrOut> = Vec::new();
        if cfg.include_pull_requests {
            let raw = match api::get_paginated(
                ctx,
                &token,
                &format!("/repos/{}/{}/pulls?state=all", context.owner, context.repo),
            ) {
                Ok(v) => v,
                Err(e) => return MiddlewareResult::failure(e),
            };
            prs = raw
                .iter()
                .map(map_pr)
                .filter(|p| {
                    p.merge_commit_sha
                        .as_deref()
                        .map(|s| shas.contains(s))
                        .unwrap_or(false)
                })
                .collect();
            prs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        }

        let mut issues: Vec<IssueOut> = Vec::new();
        if cfg.include_issues {
            // A single issue GET returns a JSON *object*, so it uses `get_json`
            // (not `get_paginated`, which expects an array).
            let mut seen: HashSet<i64> = HashSet::new();
            for pr in &prs {
                let refs = pr
                    .body
                    .as_deref()
                    .map(referenced_issues)
                    .unwrap_or_default();
                for num in refs {
                    if !seen.insert(num) {
                        continue;
                    }
                    let v = match api::get_json(
                        ctx,
                        &token,
                        &format!("/repos/{}/{}/issues/{num}", context.owner, context.repo),
                    ) {
                        Ok(v) => v,
                        Err(e) => return MiddlewareResult::failure(e),
                    };
                    issues.push(IssueOut {
                        number: v["number"].as_i64().unwrap_or(num),
                        title: s(&v, "title"),
                        body: opt(&v, "body"),
                        state: s(&v, "state"),
                        created_at: s(&v, "created_at"),
                        updated_at: s(&v, "updated_at"),
                        closed_at: opt(&v, "closed_at"),
                        pull_request_number: pr.number,
                    });
                }
            }
            issues.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        }

        MiddlewareResult::success_with(|o| {
            if !prs.is_empty() {
                o.set("prs", &prs);
                o.set("pullRequests", &prs);
            }
            if !issues.is_empty() {
                o.set("issues", &issues);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GithubShared;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn origin() -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: "https://github.com/o/r.git".into(),
        }
    }

    fn ctx_with<'a>(
        host: &'a MockHost,
        shared: &'a GithubShared,
        cfgv: &'a GithubPluginConfig,
    ) -> Context<'a> {
        Context::new(host, "/repo".into(), "s".into())
            .with_state(shared)
            .with_plugin_config(cfgv)
    }

    #[test]
    fn empty_commits_succeeds_without_http() {
        let host = MockHost::new();
        let shared = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = ctx_with(&host, &shared, &pc);
        let r = run(&RelatedItems, &ctx, RelatedItemsConfig::default());
        assert!(r.is_success());
        assert!(host.recorded_requests().is_empty());
    }

    #[test]
    fn keeps_prs_matching_merge_commit_sha_and_emits_alias() {
        let pulls = br#"[
            {"number":7,"title":"a","body":"","state":"closed","created_at":"2026-02-01T00:00:00Z","updated_at":"x","merged_at":"y","merge_commit_sha":"aaa"},
            {"number":8,"title":"b","body":"","state":"closed","created_at":"2026-03-01T00:00:00Z","updated_at":"x","merged_at":"y","merge_commit_sha":"zzz"}
        ]"#;
        let host = MockHost::new()
            .with_process_result(0, vec![origin()]) // resolve_context git remote
            .with_http_response(200, pulls); // pulls (single page)
        let shared = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = ctx_with(&host, &shared, &pc);
        let cfg = RelatedItemsConfig {
            commits: vec![CommitRef { sha: "aaa".into() }],
            include_pull_requests: true,
            include_issues: false,
        };
        let w = run(&RelatedItems, &ctx, cfg).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        let prs: serde_json::Value = serde_json::from_str(&m["prs"]).unwrap();
        assert_eq!(prs.as_array().unwrap().len(), 1);
        assert_eq!(prs[0]["number"], 7);
        assert_eq!(prs[0]["mergeCommitSha"], "aaa");
        // alias identical
        assert_eq!(m["prs"], m["pullRequests"]);
    }

    #[test]
    fn links_issue_referenced_by_pr_body() {
        let pulls = br#"[
            {"number":7,"title":"a","body":"Fixes #42","state":"closed","created_at":"2026-02-01T00:00:00Z","updated_at":"x","merged_at":"y","merge_commit_sha":"aaa"}
        ]"#;
        let issue = br#"{"number":42,"title":"bug","body":"b","state":"closed","created_at":"2026-01-01T00:00:00Z","updated_at":"x","closed_at":"z"}"#;
        let host = MockHost::new()
            .with_process_result(0, vec![origin()]) // resolve_context
            .with_http_response(200, pulls) // pulls
            .with_http_response(200, issue); // GET /issues/42
        let shared = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = ctx_with(&host, &shared, &pc);
        let cfg = RelatedItemsConfig {
            commits: vec![CommitRef { sha: "aaa".into() }],
            include_pull_requests: true,
            include_issues: true,
        };
        let w = run(&RelatedItems, &ctx, cfg).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        let issues: serde_json::Value = serde_json::from_str(&m["issues"]).unwrap();
        assert_eq!(issues.as_array().unwrap().len(), 1);
        assert_eq!(issues[0]["number"], 42);
        assert_eq!(issues[0]["pullRequestNumber"], 7);
    }
}
