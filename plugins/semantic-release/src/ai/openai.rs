//! OpenAI Chat Completions provider. Maps a normalized ChatRequest to
//! POST {base}/v1/chat/completions and back, classifying errors for the retry policy.

use moonlit_plugin_sdk::prelude::*;

use super::{AiConfig, ChatClient, ChatError, ChatRequest, ChatResponse};

pub struct OpenAiClient {
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiClient {
    pub fn new(cfg: &AiConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            model: cfg.model_or_default().to_string(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com".to_string()),
        }
    }
}

#[derive(serde::Deserialize)]
struct OaResponse {
    choices: Vec<OaChoice>,
}
#[derive(serde::Deserialize)]
struct OaChoice {
    message: OaMessage,
}
#[derive(serde::Deserialize)]
struct OaMessage {
    content: Option<String>,
}

impl ChatClient for OpenAiClient {
    fn complete(&self, ctx: &Context, req: &ChatRequest) -> Result<ChatResponse, ChatError> {
        let url = format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "developer", "content": req.system},
                {"role": "user", "content": req.user},
            ],
            "response_format": {"type": "json_object"},
        });
        let resp = ctx
            .http()
            .post(url.as_str())
            .bearer(&self.api_key)
            .json(&payload)
            .send()
            .map_err(ChatError::Transport)?;

        if resp.is_success() {
            let parsed: OaResponse = resp
                .json()
                .map_err(|e| ChatError::Malformed(format!("OpenAI response parse: {e}")))?;
            let text = parsed
                .choices
                .into_iter()
                .next()
                .and_then(|c| c.message.content)
                .map(|t| strip_fences(&t))
                .filter(|t| !t.trim().is_empty())
                .ok_or_else(|| ChatError::Malformed("OpenAI returned empty content".to_string()))?;
            return Ok(ChatResponse { text });
        }

        match resp.status() {
            401 | 403 => Err(ChatError::Auth(format!(
                "OpenAI authentication failed ({}).",
                resp.status()
            ))),
            429 => Err(ChatError::RateLimited {
                retry_after_ms: parse_retry_after(&resp),
            }),
            s => Err(ChatError::Transport(format!("OpenAI HTTP {s}"))),
        }
    }
}

/// Strip a leading ```/```json fence and a trailing ``` fence, if present.
pub fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t.strip_prefix("```json").or_else(|| t.strip_prefix("```")).unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

/// Parse a `Retry-After` header expressed in integer seconds into milliseconds.
/// HTTP-date form (non-numeric) yields `None` (the caller falls back to computed backoff).
pub fn parse_retry_after(resp: &moonlit_plugin_sdk::http::Response) -> Option<u64> {
    resp.header("retry-after")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AiConfig, ChatClient, ChatError, ChatRequest};
    use moonlit_plugin_sdk::testing::MockHost;

    fn cfg() -> AiConfig {
        moonlit_plugin_sdk::config::from_json_value(r#"{"apiKey":"sk-test"}"#).unwrap()
    }
    fn req() -> ChatRequest { ChatRequest { system: "SYS".into(), user: "USR".into() } }

    #[test]
    fn success_sends_canonical_request_and_strips_fences() {
        let body = br#"{"choices":[{"message":{"content":"```json\n{\"a\":1}\n```"}}]}"#;
        let host = MockHost::new().with_http_response(200, body);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let out = OpenAiClient::new(&cfg()).complete(&ctx, &req()).unwrap();
        assert_eq!(out.text, "{\"a\":1}");
        let r = &host.recorded_requests()[0];
        assert_eq!(r.authority, "api.openai.com");
        assert_eq!(r.path_with_query, "/v1/chat/completions");
        assert!(r.headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("authorization") && v == "Bearer sk-test"));
        let sent: serde_json::Value = serde_json::from_slice(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(sent["model"], "gpt-5-mini");
        assert_eq!(sent["messages"][0]["role"], "developer");
        assert_eq!(sent["messages"][0]["content"], "SYS");
        assert_eq!(sent["messages"][1]["role"], "user");
        assert_eq!(sent["response_format"]["type"], "json_object");
    }

    #[test]
    fn http_401_maps_to_auth() {
        let host = MockHost::new().with_http_response(401, b"nope");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(OpenAiClient::new(&cfg()).complete(&ctx, &req()), Err(ChatError::Auth(_))));
    }

    #[test]
    fn http_429_parses_retry_after_seconds() {
        let host = MockHost::new().with_http_response_headers(
            429, vec![("retry-after".into(), "2".into())], b"slow down");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        match OpenAiClient::new(&cfg()).complete(&ctx, &req()) {
            Err(ChatError::RateLimited { retry_after_ms }) => assert_eq!(retry_after_ms, Some(2000)),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn empty_content_is_malformed() {
        let host = MockHost::new().with_http_response(200, br#"{"choices":[]}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(OpenAiClient::new(&cfg()).complete(&ctx, &req()), Err(ChatError::Malformed(_))));
    }

    #[test]
    fn http_500_maps_to_transport() {
        let host = MockHost::new().with_http_response(500, b"boom");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(OpenAiClient::new(&cfg()).complete(&ctx, &req()), Err(ChatError::Transport(_))));
    }
}
