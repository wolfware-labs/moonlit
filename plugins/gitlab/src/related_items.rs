//! `gitlab related-items` — merged MRs (and their closed issues) for a commit set.

use std::collections::HashSet;

use moonlit_plugin_sdk::prelude::*;
use serde::Serialize;
use serde_json::Value;

use crate::api;
use crate::config::GitlabPluginConfig;
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
    #[serde(alias = "includePullRequests")]
    include_merge_requests: bool,
    include_issues: bool,
}

impl Default for RelatedItemsConfig {
    fn default() -> Self {
        Self {
            commits: Vec::new(),
            include_merge_requests: true,
            include_issues: true,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MrOut {
    iid: i64,
    title: String,
    description: Option<String>,
    state: String,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IssueOut {
    iid: i64,
    title: String,
    description: Option<String>,
    state: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    merge_request_iid: i64,
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").to_string()
}
fn opt(v: &Value, k: &str) -> Option<String> {
    v[k].as_str().map(String::from)
}

fn map_mr(v: &Value) -> MrOut {
    MrOut {
        iid: v["iid"].as_i64().unwrap_or(0),
        title: s(v, "title"),
        description: opt(v, "description"),
        state: s(v, "state"),
        created_at: s(v, "created_at"),
        updated_at: s(v, "updated_at"),
        merged_at: opt(v, "merged_at"),
        merge_commit_sha: opt(v, "merge_commit_sha"),
    }
}

#[derive(Default)]
pub struct RelatedItems;

impl Middleware for RelatedItems {
    const NAME: &'static str = "related-items";
    const DESCRIPTION: &'static str = "merged MRs and closed issues for a commit set";
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
        let api_base = context.api_base();
        let token = ctx.plugin_config::<GitlabPluginConfig>().token.clone();
        let shas: HashSet<String> = cfg.commits.into_iter().map(|c| c.sha).collect();

        // Fetch raw merged MRs so we can match on BOTH merge_commit_sha and
        // squash_commit_sha (squash-merged MRs record only the latter).
        let mut mrs: Vec<MrOut> = Vec::new();
        if cfg.include_merge_requests {
            let raw = match api::get_paginated(
                ctx,
                &api_base,
                &token,
                &format!(
                    "/projects/{}/merge_requests?state=merged",
                    context.project_id
                ),
            ) {
                Ok(v) => v,
                Err(e) => return MiddlewareResult::failure(e),
            };
            mrs = raw
                .iter()
                .filter(|v| {
                    let merged = v["merge_commit_sha"]
                        .as_str()
                        .map(|s| shas.contains(s))
                        .unwrap_or(false);
                    let squashed = v["squash_commit_sha"]
                        .as_str()
                        .map(|s| shas.contains(s))
                        .unwrap_or(false);
                    merged || squashed
                })
                .map(map_mr)
                .collect();
            mrs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        }

        let mut issues: Vec<IssueOut> = Vec::new();
        if cfg.include_issues {
            // GitLab's native linked-issues endpoint: GET .../merge_requests/:iid/closes_issues.
            let mut seen: HashSet<i64> = HashSet::new();
            for mr in &mrs {
                let closed = match api::get_paginated(
                    ctx,
                    &api_base,
                    &token,
                    &format!(
                        "/projects/{}/merge_requests/{}/closes_issues",
                        context.project_id, mr.iid
                    ),
                ) {
                    Ok(v) => v,
                    Err(e) => return MiddlewareResult::failure(e),
                };
                for v in &closed {
                    let iid = v["iid"].as_i64().unwrap_or(0);
                    if !seen.insert(iid) {
                        continue;
                    }
                    issues.push(IssueOut {
                        iid,
                        title: s(v, "title"),
                        description: opt(v, "description"),
                        state: s(v, "state"),
                        created_at: s(v, "created_at"),
                        updated_at: s(v, "updated_at"),
                        closed_at: opt(v, "closed_at"),
                        merge_request_iid: mr.iid,
                    });
                }
            }
            issues.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        }

        MiddlewareResult::success_with(|o| {
            if !mrs.is_empty() {
                o.set("mrs", &mrs);
                o.set("prs", &mrs);
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
    use crate::context::GitlabShared;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn origin() -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: "https://gitlab.com/o/r.git".into(),
        }
    }
    fn pc() -> GitlabPluginConfig {
        GitlabPluginConfig {
            token: "t".into(),
            base_url: "https://gitlab.com".into(),
        }
    }
    fn ctx_with<'a>(
        host: &'a MockHost,
        sh: &'a GitlabShared,
        cfg: &'a GitlabPluginConfig,
    ) -> Context<'a> {
        Context::new(host, "/repo".into(), "s".into())
            .with_state(sh)
            .with_plugin_config(cfg)
    }

    #[test]
    fn empty_commits_succeeds_without_http() {
        let host = MockHost::new();
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let r = run(&RelatedItems, &ctx, RelatedItemsConfig::default());
        assert!(r.is_success());
        assert!(host.recorded_requests().is_empty());
    }

    #[test]
    fn keeps_mrs_matching_merge_commit_sha_and_emits_prs_alias() {
        let mrs = br#"[
            {"iid":7,"title":"a","description":"","state":"merged","created_at":"2026-02-01T00:00:00Z","updated_at":"x","merged_at":"y","merge_commit_sha":"aaa"},
            {"iid":8,"title":"b","description":"","state":"merged","created_at":"2026-03-01T00:00:00Z","updated_at":"x","merged_at":"y","merge_commit_sha":"zzz"}
        ]"#;
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(200, mrs);
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = RelatedItemsConfig {
            commits: vec![CommitRef { sha: "aaa".into() }],
            include_merge_requests: true,
            include_issues: false,
        };
        let w = run(&RelatedItems, &ctx, config).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        let out: serde_json::Value = serde_json::from_str(&m["mrs"]).unwrap();
        assert_eq!(out.as_array().unwrap().len(), 1);
        assert_eq!(out[0]["iid"], 7);
        assert_eq!(out[0]["mergeCommitSha"], "aaa");
        assert_eq!(m["mrs"], m["prs"]); // alias identical
                                        // GitLab MR API path with URL-encoded project id.
        assert_eq!(
            host.recorded_requests()[0].path_with_query,
            "/api/v4/projects/o%2Fr/merge_requests?state=merged&per_page=100"
        );
    }

    #[test]
    fn matches_squash_commit_sha() {
        let mrs = br#"[
            {"iid":9,"title":"c","description":null,"state":"merged","created_at":"2026-04-01T00:00:00Z","updated_at":"x","merged_at":"y","squash_commit_sha":"sqsq"}
        ]"#;
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(200, mrs);
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = RelatedItemsConfig {
            commits: vec![CommitRef { sha: "sqsq".into() }],
            include_merge_requests: true,
            include_issues: false,
        };
        let w = run(&RelatedItems, &ctx, config).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        let out: serde_json::Value = serde_json::from_str(&m["mrs"]).unwrap();
        assert_eq!(out.as_array().unwrap().len(), 1);
        assert_eq!(out[0]["iid"], 9);
    }

    #[test]
    fn links_closed_issues_via_endpoint() {
        let mrs = br#"[
            {"iid":7,"title":"a","description":"","state":"merged","created_at":"2026-02-01T00:00:00Z","updated_at":"x","merged_at":"y","merge_commit_sha":"aaa"}
        ]"#;
        let closes = br#"[
            {"iid":42,"title":"bug","description":"b","state":"closed","created_at":"2026-01-01T00:00:00Z","updated_at":"x","closed_at":"z"}
        ]"#;
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(200, mrs) // merge_requests?state=merged
            .with_http_response(200, closes); // merge_requests/7/closes_issues
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = RelatedItemsConfig {
            commits: vec![CommitRef { sha: "aaa".into() }],
            include_merge_requests: true,
            include_issues: true,
        };
        let w = run(&RelatedItems, &ctx, config).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        let issues: serde_json::Value = serde_json::from_str(&m["issues"]).unwrap();
        assert_eq!(issues.as_array().unwrap().len(), 1);
        assert_eq!(issues[0]["iid"], 42);
        assert_eq!(issues[0]["mergeRequestIid"], 7);
        assert_eq!(
            host.recorded_requests()[1].path_with_query,
            "/api/v4/projects/o%2Fr/merge_requests/7/closes_issues?per_page=100"
        );
    }

    #[test]
    fn include_pull_requests_alias_is_accepted() {
        // The camelCase alias `includePullRequests` must deserialize into include_merge_requests.
        let cfg: RelatedItemsConfig = serde_json::from_str(
            r#"{"commits":[],"includePullRequests":false,"includeIssues":false}"#,
        )
        .unwrap();
        assert!(!cfg.include_merge_requests);
        assert!(!cfg.include_issues);
    }
}
