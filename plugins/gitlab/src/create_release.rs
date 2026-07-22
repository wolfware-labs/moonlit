//! `gitlab create-release` — create a GitLab release, then comment/label related items.

use moonlit_plugin_sdk::changelog::{self, Category};
use moonlit_plugin_sdk::prelude::*;
use serde_json::{json, Value};

use crate::api;
use crate::config::GitlabPluginConfig;
use crate::context::{resolve_context, GitlabContext};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemRef {
    #[serde(alias = "number")]
    iid: i64,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateReleaseConfig {
    name: String,
    tag: String,
    label: Option<String>,
    body: Option<String>,
    changelog: Vec<Category>,
    draft: bool,
    prerelease: bool,
    #[serde(alias = "pullRequests")]
    merge_requests: Vec<ItemRef>,
    issues: Vec<ItemRef>,
}

#[derive(Default)]
pub struct CreateRelease;

impl Middleware for CreateRelease {
    const NAME: &'static str = "create-release";
    const DESCRIPTION: &'static str = "create a GitLab release and annotate related items";
    type Config = CreateReleaseConfig;

    fn execute(&self, ctx: &Context, cfg: CreateReleaseConfig) -> MiddlewareResult {
        if cfg.name.trim().is_empty() {
            return MiddlewareResult::failure("Release name is required.");
        }
        if cfg.tag.trim().is_empty() {
            return MiddlewareResult::failure("Release tag is required.");
        }
        let body_blank = cfg.body.as_deref().map(str::trim).unwrap_or("").is_empty();
        if body_blank && cfg.changelog.is_empty() {
            return MiddlewareResult::failure("Release body or changelog is required.");
        }
        if cfg.draft {
            ctx.log_warn("GitLab does not support draft releases; ignoring the draft flag.");
        }
        let context = match resolve_context(ctx) {
            Ok(c) => c,
            Err(f) => return f,
        };
        let api_base = context.api_base();
        let token = ctx.plugin_config::<GitlabPluginConfig>().token.clone();

        // GitLab has no native prerelease flag: suffix the name instead.
        let release_name = if cfg.prerelease {
            format!("{} (pre-release)", cfg.name)
        } else {
            cfg.name.clone()
        };

        let body = if body_blank {
            changelog::render(&cfg.changelog, &context.commit_url_prefix())
        } else {
            cfg.body.clone().unwrap_or_default()
        };

        let payload = json!({
            "name": release_name,
            "tag_name": cfg.tag,
            "ref": "HEAD",
            "description": body,
        });
        let resp = match api::post_json(
            ctx,
            &api_base,
            &token,
            &format!("/projects/{}/releases", context.project_id),
            &payload,
        ) {
            Ok(r) => r,
            Err(e) => return MiddlewareResult::failure(e),
        };
        let created: Value = match resp.json() {
            Ok(v) => v,
            Err(e) => return MiddlewareResult::failure(e),
        };
        let out_name = created["name"].as_str().unwrap_or(&release_name).to_string();
        let out_url = created["_links"]["self"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| format!("{}/-/releases/{}", context.web_url(), cfg.tag));

        // Comment + optional label on each related MR/issue (warn-and-continue).
        // Same comment body text as the github plugin.
        let comment = format!(
            ":rocket: **New Release Published!**\n\n\
             :tada: A new version of the project has just been released!\n\n\
             **:bookmark: Link:** [`{}`]({})",
            out_name, out_url
        );
        annotate(ctx, &api_base, &token, &context, "merge_requests", "merge request", &cfg.merge_requests, &comment, cfg.label.as_deref());
        annotate(ctx, &api_base, &token, &context, "issues", "issue", &cfg.issues, &comment, cfg.label.as_deref());

        MiddlewareResult::success_with(|o| {
            o.set("name", out_name);
            o.set("url", out_url);
        })
    }
}

/// Comment on and optionally label each item. MRs and issues have different API
/// path segments (`merge_requests` vs `issues`), so the collection + display kind
/// are passed in. Failures warn and continue (never fail the release).
#[allow(clippy::too_many_arguments)]
fn annotate(
    ctx: &Context,
    api_base: &str,
    token: &str,
    context: &GitlabContext,
    collection: &str,
    kind: &str,
    items: &[ItemRef],
    comment: &str,
    label: Option<&str>,
) {
    for item in items {
        let note_path = format!("/projects/{}/{}/{}/notes", context.project_id, collection, item.iid);
        if let Err(e) = api::post_json(ctx, api_base, token, &note_path, &json!({ "body": comment })) {
            ctx.log_warn(&format!("Failed to comment on {kind} {}: {e}", item.iid));
        }
        if let Some(label) = label {
            let lpath = format!(
                "/projects/{}/{}/{}?add_labels={}",
                context.project_id,
                collection,
                item.iid,
                encode_query(label),
            );
            if let Err(e) = api::put(ctx, api_base, token, &lpath) {
                ctx.log_warn(&format!("Failed to label {kind} {}: {e}", item.iid));
            }
        }
    }
}

/// Percent-encode a query-parameter value (labels may contain spaces, etc.). The
/// unreserved set is RFC 3986's `A-Za-z0-9-._~`; everything else is `%XX`.
fn encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GitlabShared;
    use moonlit_plugin_sdk::changelog::Entry;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn origin() -> OutputChunk {
        OutputChunk { stream: StdioStream::Stdout, text: "https://gitlab.com/o/r.git".into() }
    }
    fn pc() -> GitlabPluginConfig {
        GitlabPluginConfig { token: "t".into(), base_url: "https://gitlab.com".into() }
    }
    fn ctx_with<'a>(host: &'a MockHost, sh: &'a GitlabShared, cfg: &'a GitlabPluginConfig) -> Context<'a> {
        Context::new(host, "/repo".into(), "s".into()).with_state(sh).with_plugin_config(cfg)
    }

    #[test]
    fn blank_name_fails_before_http() {
        let host = MockHost::new();
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig { tag: "v1".into(), body: Some("x".into()), ..Default::default() };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(!w.successful);
        assert_eq!(w.error_message.as_deref(), Some("Release name is required."));
        assert!(host.recorded_requests().is_empty());
    }

    #[test]
    fn blank_body_and_empty_changelog_fails() {
        let host = MockHost::new();
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig { name: "1.0".into(), tag: "v1".into(), ..Default::default() };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(!w.successful);
        assert_eq!(w.error_message.as_deref(), Some("Release body or changelog is required."));
    }

    #[test]
    fn creates_release_and_emits_name_and_url() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(201, br#"{"name":"1.0.0","_links":{"self":"https://gitlab.com/o/r/-/releases/v1.0.0"}}"#);
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(m["name"], "\"1.0.0\"");
        assert_eq!(m["url"], "\"https://gitlab.com/o/r/-/releases/v1.0.0\"");
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].path_with_query, "/api/v4/projects/o%2Fr/releases");
        let body: serde_json::Value = serde_json::from_slice(reqs[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(body["tag_name"], "v1.0.0");
        assert_eq!(body["ref"], "HEAD");
        assert_eq!(body["name"], "1.0.0");
        assert_eq!(body["description"], "notes");
    }

    #[test]
    fn prerelease_appends_marker_to_name() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(201, br#"{"name":"1.0.0 (pre-release)","_links":{"self":"u"}}"#);
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            prerelease: true,
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(w.successful);
        let reqs = host.recorded_requests();
        let body: serde_json::Value = serde_json::from_slice(reqs[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(body["name"], "1.0.0 (pre-release)");
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(m["name"], "\"1.0.0 (pre-release)\"");
    }

    #[test]
    fn draft_warns_and_is_ignored() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(201, br#"{"name":"1.0.0","_links":{"self":"u"}}"#);
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            draft: true,
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(w.successful);
        assert!(host.logs().iter().any(|(_, m)| m == "GitLab does not support draft releases; ignoring the draft flag."));
    }

    #[test]
    fn absent_body_renders_changelog_into_description() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(201, br#"{"name":"1.0.0","_links":{"self":"u"}}"#);
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: None,
            changelog: vec![Category {
                name: "Features".into(),
                icon: ":sparkles:".into(),
                summary: "New".into(),
                entries: vec![Entry { sha: "abcdef1234".into(), description: "y".into() }],
            }],
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(w.successful);
        let reqs = host.recorded_requests();
        let body: serde_json::Value = serde_json::from_slice(reqs[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(
            body["description"],
            "## :sparkles: Features\n#### New\n- y ([abcdef1](https://gitlab.com/o/r/-/commit/abcdef1234))\n\n"
        );
    }

    #[test]
    fn comments_on_related_merge_request() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(201, br#"{"name":"1.0.0","_links":{"self":"https://gitlab.com/o/r/-/releases/v1.0.0"}}"#) // release POST
            .with_http_response(201, b"{}"); // MR note POST
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            merge_requests: vec![ItemRef { iid: 7 }],
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(w.successful);
        let reqs = host.recorded_requests();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[1].path_with_query, "/api/v4/projects/o%2Fr/merge_requests/7/notes");
        let cbody: serde_json::Value = serde_json::from_slice(reqs[1].body.as_deref().unwrap()).unwrap();
        assert_eq!(
            cbody["body"],
            ":rocket: **New Release Published!**\n\n:tada: A new version of the project has just been released!\n\n**:bookmark: Link:** [`1.0.0`](https://gitlab.com/o/r/-/releases/v1.0.0)"
        );
    }

    #[test]
    fn labels_issue_via_type_specific_path() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()])
            .with_http_response(201, br#"{"name":"1.0.0","_links":{"self":"u"}}"#) // release POST
            .with_http_response(201, b"{}") // issue note POST
            .with_http_response(200, b"{}"); // issue label PUT
        let sh = GitlabShared::default();
        let cfg = pc();
        let ctx = ctx_with(&host, &sh, &cfg);
        let config = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            label: Some("released".into()),
            issues: vec![ItemRef { iid: 9 }],
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, config).into_wit();
        assert!(w.successful);
        let reqs = host.recorded_requests();
        assert_eq!(reqs[1].path_with_query, "/api/v4/projects/o%2Fr/issues/9/notes");
        assert_eq!(reqs[2].path_with_query, "/api/v4/projects/o%2Fr/issues/9?add_labels=released");
    }

    #[test]
    fn item_ref_accepts_number_alias() {
        let r: ItemRef = serde_json::from_str(r#"{"number":5}"#).unwrap();
        assert_eq!(r.iid, 5);
    }
}
