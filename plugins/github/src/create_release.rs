//! `github create-release` — create a GitHub Release, then comment/label related items.

use moonlit_sdk::changelog::{self, Category};
use moonlit_sdk::prelude::*;
use serde_json::json;

use crate::api;
use crate::config::GithubPluginConfig;
use crate::context::resolve_context;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemRef {
    number: i64,
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
    pull_requests: Vec<ItemRef>,
    issues: Vec<ItemRef>,
}

#[derive(Default)]
pub struct CreateRelease;

impl Middleware for CreateRelease {
    const NAME: &'static str = "create-release";
    const DESCRIPTION: &'static str = "create a GitHub release and annotate related items";
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
        let context = match resolve_context(ctx) {
            Ok(c) => c,
            Err(f) => return f,
        };
        let token = ctx.plugin_config::<GithubPluginConfig>().token.clone();

        let body = if body_blank {
            changelog::render(&cfg.changelog, &context.commit_url_prefix())
        } else {
            cfg.body.clone().unwrap_or_default()
        };

        let payload = json!({
            "tag_name": cfg.tag,
            "name": cfg.name,
            "body": body,
            "draft": cfg.draft,
            "prerelease": cfg.prerelease,
        });
        let resp = match api::post_json(
            ctx,
            &token,
            &format!("/repos/{}/{}/releases", context.owner, context.repo),
            &payload,
        ) {
            Ok(r) => r,
            Err(e) => return MiddlewareResult::failure(e),
        };
        let created: serde_json::Value = match resp.json() {
            Ok(v) => v,
            Err(e) => return MiddlewareResult::failure(e),
        };
        let out_name = created["name"].as_str().unwrap_or(&cfg.name).to_string();
        let out_url = created["html_url"].as_str().unwrap_or("").to_string();

        // Comment + optional label on each related PR/issue (warn-and-continue).
        // Byte-for-byte 1.x `GetReleaseComment` text (no trailing newline).
        let comment = format!(
            ":rocket: **New Release Published!**\n\n\
             :tada: A new version of the project has just been released!\n\n\
             **:bookmark: Link:** [`{}`]({})",
            out_name, out_url
        );
        let numbers: Vec<i64> = cfg
            .pull_requests
            .iter()
            .chain(cfg.issues.iter())
            .map(|i| i.number)
            .collect();
        for number in numbers {
            let cpath = format!(
                "/repos/{}/{}/issues/{number}/comments",
                context.owner, context.repo
            );
            if let Err(e) = api::post_json(ctx, &token, &cpath, &json!({ "body": comment })) {
                ctx.log_warn(&format!("Failed to comment on #{number}: {e}"));
            }
            if let Some(label) = &cfg.label {
                let lpath = format!(
                    "/repos/{}/{}/issues/{number}/labels",
                    context.owner, context.repo
                );
                if let Err(e) = api::post_json(ctx, &token, &lpath, &json!({ "labels": [label] })) {
                    ctx.log_warn(&format!("Failed to label #{number}: {e}"));
                }
            }
        }

        MiddlewareResult::success_with(|o| {
            o.set("name", out_name);
            o.set("url", out_url);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::GithubShared;
    use moonlit_sdk::changelog::Entry;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::{run, MockHost};

    fn origin() -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: "https://github.com/o/r.git".into(),
        }
    }
    fn base<'a>(
        host: &'a MockHost,
        sh: &'a GithubShared,
        pc: &'a GithubPluginConfig,
    ) -> Context<'a> {
        Context::new(host, "/repo".into(), "s".into())
            .with_state(sh)
            .with_plugin_config(pc)
    }

    #[test]
    fn blank_name_fails_before_http() {
        let host = MockHost::new();
        let sh = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = base(&host, &sh, &pc);
        let cfg = CreateReleaseConfig {
            tag: "v1".into(),
            body: Some("x".into()),
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Release name is required.")
        );
        assert!(host.recorded_requests().is_empty());
    }

    #[test]
    fn blank_body_and_empty_changelog_fails() {
        let host = MockHost::new();
        let sh = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = base(&host, &sh, &pc);
        let cfg = CreateReleaseConfig {
            name: "1.0".into(),
            tag: "v1".into(),
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Release body or changelog is required.")
        );
    }

    #[test]
    fn creates_release_and_emits_name_and_url() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()]) // resolve_context
            .with_http_response(
                201,
                br#"{"name":"1.0.0","html_url":"https://github.com/o/r/releases/1"}"#,
            );
        let sh = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = base(&host, &sh, &pc);
        let cfg = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, cfg).into_wit();
        assert!(w.successful);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(m["name"], "\"1.0.0\"");
        assert_eq!(m["url"], "\"https://github.com/o/r/releases/1\"");
        // release POST body carried our fields
        let reqs = host.recorded_requests();
        let body: serde_json::Value =
            serde_json::from_slice(reqs[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(body["tag_name"], "v1.0.0");
        assert_eq!(body["draft"], false);
    }

    #[test]
    fn absent_body_renders_changelog_into_release_body() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()]) // resolve_context
            .with_http_response(
                201,
                br#"{"name":"1.0.0","html_url":"https://github.com/o/r/releases/1"}"#,
            );
        let sh = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = base(&host, &sh, &pc);
        let cfg = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: None,
            changelog: vec![Category {
                name: "Features".into(),
                icon: ":sparkles:".into(),
                summary: "New".into(),
                entries: vec![Entry {
                    sha: "abcdef1234".into(),
                    description: "y".into(),
                }],
            }],
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, cfg).into_wit();
        assert!(w.successful);
        // The release POST body should equal changelog::render with the repo's commit prefix.
        let reqs = host.recorded_requests();
        let body: serde_json::Value =
            serde_json::from_slice(reqs[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(
            body["body"],
            "## :sparkles: Features\n#### New\n- y ([abcdef1](https://github.com/o/r/commit/abcdef1234))\n\n"
        );
    }

    #[test]
    fn comments_on_related_pull_request() {
        let host = MockHost::new()
            .with_process_result(0, vec![origin()]) // resolve_context
            .with_http_response(
                201,
                br#"{"name":"1.0.0","html_url":"https://github.com/o/r/releases/1"}"#,
            ) // release POST
            .with_http_response(201, b"{}"); // comment POST
        let sh = GithubShared::default();
        let pc = GithubPluginConfig { token: "t".into() };
        let ctx = base(&host, &sh, &pc);
        let cfg = CreateReleaseConfig {
            name: "1.0.0".into(),
            tag: "v1.0.0".into(),
            body: Some("notes".into()),
            pull_requests: vec![ItemRef { number: 7 }],
            ..Default::default()
        };
        let w = run(&CreateRelease, &ctx, cfg).into_wit();
        assert!(w.successful);
        let reqs = host.recorded_requests();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[1].path_with_query, "/repos/o/r/issues/7/comments");
        let cbody: serde_json::Value =
            serde_json::from_slice(reqs[1].body.as_deref().unwrap()).unwrap();
        assert_eq!(
            cbody["body"],
            ":rocket: **New Release Published!**\n\n:tada: A new version of the project has just been released!\n\n**:bookmark: Link:** [`1.0.0`](https://github.com/o/r/releases/1)"
        );
    }
}
