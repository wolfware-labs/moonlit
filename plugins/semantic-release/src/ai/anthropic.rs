//! Anthropic Messages API provider. Maps a normalized ChatRequest to
//! POST {base}/v1/messages and back, classifying errors for the retry policy.

use moonlit_sdk::prelude::*;

use super::openai::{parse_retry_after, strip_fences};
use super::{AiConfig, ChatClient, ChatError, ChatRequest, ChatResponse};

pub struct AnthropicClient {
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
}

impl AnthropicClient {
    pub fn new(cfg: &AiConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            model: cfg.model_or_default().to_string(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            max_tokens: cfg.max_tokens_or_default(),
        }
    }
}

#[derive(serde::Deserialize)]
struct AnthResponse {
    content: Vec<AnthBlock>,
}
#[derive(serde::Deserialize)]
struct AnthBlock {
    text: Option<String>,
}

impl ChatClient for AnthropicClient {
    fn complete(&self, ctx: &Context, req: &ChatRequest) -> Result<ChatResponse, ChatError> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": req.system,
            "messages": [
                {"role": "user", "content": req.user},
            ],
        });
        let resp = ctx
            .http()
            .post(url.as_str())
            .header("x-api-key", self.api_key.as_str())
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .map_err(ChatError::Transport)?;

        if resp.is_success() {
            let parsed: AnthResponse = resp
                .json()
                .map_err(|e| ChatError::Malformed(format!("Anthropic response parse: {e}")))?;
            let text = parsed
                .content
                .into_iter()
                .find_map(|b| b.text)
                .map(|t| strip_fences(&t))
                .filter(|t| !t.trim().is_empty())
                .ok_or_else(|| {
                    ChatError::Malformed("Anthropic returned empty content".to_string())
                })?;
            return Ok(ChatResponse { text });
        }

        match resp.status() {
            401 | 403 => Err(ChatError::Auth(format!(
                "Anthropic authentication failed ({}).",
                resp.status()
            ))),
            429 => Err(ChatError::RateLimited {
                retry_after_ms: parse_retry_after(&resp),
            }),
            s => Err(ChatError::Transport(format!("Anthropic HTTP {s}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AiConfig, ChatClient, ChatError, ChatRequest};
    use moonlit_sdk::testing::MockHost;

    fn cfg() -> AiConfig {
        moonlit_sdk::config::from_json_value(r#"{"provider":"anthropic","apiKey":"ak-test"}"#)
            .unwrap()
    }
    fn req() -> ChatRequest {
        ChatRequest { system: "SYS".into(), user: "USR".into() }
    }

    #[test]
    fn success_sends_canonical_request_and_strips_fences() {
        let body = br#"{"content":[{"type":"text","text":"```json\n{\"a\":1}\n```"}]}"#;
        let host = MockHost::new().with_http_response(200, body);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let out = AnthropicClient::new(&cfg()).complete(&ctx, &req()).unwrap();
        assert_eq!(out.text, "{\"a\":1}");
        let r = &host.recorded_requests()[0];
        assert_eq!(r.authority, "api.anthropic.com");
        assert_eq!(r.path_with_query, "/v1/messages");
        assert!(r
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("x-api-key") && v == "ak-test"));
        assert!(r
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("anthropic-version") && v == "2023-06-01"));
        // The API key must never travel as a bearer token.
        assert!(!r.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("authorization")));
        let sent: serde_json::Value = serde_json::from_slice(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(sent["model"], "claude-haiku-4-5");
        assert_eq!(sent["max_tokens"], 4096);
        assert_eq!(sent["system"], "SYS");
        assert_eq!(sent["messages"][0]["role"], "user");
        assert_eq!(sent["messages"][0]["content"], "USR");
    }

    #[test]
    fn http_401_maps_to_auth() {
        let host = MockHost::new().with_http_response(401, b"nope");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(
            AnthropicClient::new(&cfg()).complete(&ctx, &req()),
            Err(ChatError::Auth(_))
        ));
    }

    #[test]
    fn http_429_parses_retry_after_seconds() {
        let host = MockHost::new().with_http_response_headers(
            429,
            vec![("retry-after".into(), "2".into())],
            b"slow down",
        );
        let ctx = Context::new(&host, "/w".into(), "s".into());
        match AnthropicClient::new(&cfg()).complete(&ctx, &req()) {
            Err(ChatError::RateLimited { retry_after_ms }) => assert_eq!(retry_after_ms, Some(2000)),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn empty_content_is_malformed() {
        let host = MockHost::new().with_http_response(200, br#"{"content":[]}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(
            AnthropicClient::new(&cfg()).complete(&ctx, &req()),
            Err(ChatError::Malformed(_))
        ));
    }

    #[test]
    fn http_500_maps_to_transport() {
        let host = MockHost::new().with_http_response(500, b"boom");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(
            AnthropicClient::new(&cfg()).complete(&ctx, &req()),
            Err(ChatError::Transport(_))
        ));
    }
}
