//! Thin Slack Web API client over `ctx.http()`. Slack is "always-200": logical
//! failures come back as HTTP 200 with `{"ok":false,"error":"<code>"}`, so this
//! branches on the parsed `ok` field, not the HTTP status.

use moonlit_plugin_sdk::prelude::*;

const BASE: &str = "https://slack.com/api";

#[derive(serde::Deserialize)]
struct SlackResponse {
    ok: bool,
    error: Option<String>,
}

/// POST `chat.postMessage`. Ok(()) only when the parsed body has `ok: true`.
pub fn post_message(ctx: &Context, token: &str, channel: &str, text: &str) -> Result<(), String> {
    let url = format!("{BASE}/chat.postMessage");
    let resp = match ctx
        .http()
        .post(url.as_str())
        .bearer(token)
        .json(&serde_json::json!({ "channel": channel, "text": text }))
        .send()
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to send Slack notification: {e}")),
    };
    match resp.json::<SlackResponse>() {
        Ok(body) if body.ok => Ok(()),
        Ok(body) => Err(format!(
            "Failed to send Slack notification: {}",
            body.error.as_deref().unwrap_or("unknown")
        )),
        Err(_) => Err(format!(
            "Failed to send Slack notification: HTTP {}",
            resp.status()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::MockHost;

    #[test]
    fn success_sends_canonical_request() {
        let host = MockHost::new().with_http_response(200, br#"{"ok":true}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        post_message(&ctx, "xoxb-tok", "#general", "hello").unwrap();
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].authority, "slack.com");
        assert_eq!(reqs[0].path_with_query, "/api/chat.postMessage");
        let has = |k: &str, v: &str| {
            reqs[0].headers.iter().any(|(hk, hv)| hk.eq_ignore_ascii_case(k) && hv == v)
        };
        assert!(has("authorization", "Bearer xoxb-tok"));
        let body: serde_json::Value =
            serde_json::from_slice(reqs[0].body.as_deref().unwrap()).unwrap();
        assert_eq!(body["channel"], "#general");
        assert_eq!(body["text"], "hello");
    }

    #[test]
    fn ok_false_surfaces_error_code() {
        let host = MockHost::new().with_http_response(200, br#"{"ok":false,"error":"channel_not_found"}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let e = match post_message(&ctx, "t", "#x", "m") {
            Ok(()) => panic!("ok:false must fail"),
            Err(e) => e,
        };
        assert_eq!(e, "Failed to send Slack notification: channel_not_found");
    }

    #[test]
    fn ok_false_without_error_is_unknown() {
        let host = MockHost::new().with_http_response(200, br#"{"ok":false}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let e = match post_message(&ctx, "t", "#x", "m") {
            Ok(()) => panic!("ok:false must fail"),
            Err(e) => e,
        };
        assert_eq!(e, "Failed to send Slack notification: unknown");
    }

    #[test]
    fn unparseable_body_reports_http_status() {
        let host = MockHost::new().with_http_response(500, b"<html>gateway</html>");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let e = match post_message(&ctx, "t", "#x", "m") {
            Ok(()) => panic!("500 must fail"),
            Err(e) => e,
        };
        assert_eq!(e, "Failed to send Slack notification: HTTP 500");
    }
}
