//! Google Gemini (Generative Language) provider. Maps a normalized ChatRequest to
//! POST {base}/v1beta/models/{model}:generateContent and back, classifying errors
//! for the retry policy. The API key travels in the `x-goog-api-key` header (never
//! in the URL, so it can't leak through request logs).

use moonlit_sdk::prelude::*;

use super::openai::{parse_retry_after, strip_fences};
use super::{AiConfig, ChatClient, ChatError, ChatRequest, ChatResponse};

pub struct GeminiClient {
    api_key: String,
    model: String,
    base_url: String,
}

impl GeminiClient {
    pub fn new(cfg: &AiConfig) -> Self {
        Self {
            api_key: cfg.api_key.clone(),
            model: cfg.model_or_default().to_string(),
            base_url: cfg
                .base_url
                .clone()
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string()),
        }
    }
}

#[derive(serde::Deserialize)]
struct GemResponse {
    candidates: Vec<GemCandidate>,
}
#[derive(serde::Deserialize)]
struct GemCandidate {
    content: GemContent,
}
#[derive(serde::Deserialize)]
struct GemContent {
    parts: Vec<GemPart>,
}
#[derive(serde::Deserialize)]
struct GemPart {
    text: Option<String>,
}

impl ChatClient for GeminiClient {
    fn complete(&self, ctx: &Context, req: &ChatRequest) -> Result<ChatResponse, ChatError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url.trim_end_matches('/'),
            self.model
        );
        let payload = serde_json::json!({
            "system_instruction": {"parts": [{"text": req.system}]},
            "contents": [{"role": "user", "parts": [{"text": req.user}]}],
            "generationConfig": {"responseMimeType": "application/json"},
        });
        let resp = ctx
            .http()
            .post(url.as_str())
            .header("x-goog-api-key", self.api_key.as_str())
            .json(&payload)
            .send()
            .map_err(ChatError::Transport)?;

        if resp.is_success() {
            let parsed: GemResponse = resp
                .json()
                .map_err(|e| ChatError::Malformed(format!("Gemini response parse: {e}")))?;
            let text = parsed
                .candidates
                .into_iter()
                .next()
                .and_then(|c| c.content.parts.into_iter().find_map(|p| p.text))
                .map(|t| strip_fences(&t))
                .filter(|t| !t.trim().is_empty())
                .ok_or_else(|| ChatError::Malformed("Gemini returned empty content".to_string()))?;
            return Ok(ChatResponse { text });
        }

        match resp.status() {
            401 | 403 => Err(ChatError::Auth(format!(
                "Gemini authentication failed ({}).",
                resp.status()
            ))),
            429 => Err(ChatError::RateLimited {
                retry_after_ms: parse_retry_after(&resp),
            }),
            s => Err(ChatError::Transport(format!("Gemini HTTP {s}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AiConfig, ChatClient, ChatError, ChatRequest};
    use moonlit_sdk::testing::MockHost;

    fn cfg() -> AiConfig {
        moonlit_sdk::config::from_json_value(r#"{"provider":"gemini","apiKey":"gk-test"}"#)
            .unwrap()
    }
    fn req() -> ChatRequest {
        ChatRequest { system: "SYS".into(), user: "USR".into() }
    }

    #[test]
    fn success_sends_canonical_request_and_strips_fences() {
        let body = br#"{"candidates":[{"content":{"parts":[{"text":"```json\n{\"a\":1}\n```"}]}}]}"#;
        let host = MockHost::new().with_http_response(200, body);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let out = GeminiClient::new(&cfg()).complete(&ctx, &req()).unwrap();
        assert_eq!(out.text, "{\"a\":1}");
        let r = &host.recorded_requests()[0];
        assert_eq!(r.authority, "generativelanguage.googleapis.com");
        // Model is in the path; the API key is a header, not a query param.
        assert_eq!(r.path_with_query, "/v1beta/models/gemini-2.5-flash:generateContent");
        assert!(r
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("x-goog-api-key") && v == "gk-test"));
        let sent: serde_json::Value = serde_json::from_slice(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(sent["system_instruction"]["parts"][0]["text"], "SYS");
        assert_eq!(sent["contents"][0]["role"], "user");
        assert_eq!(sent["contents"][0]["parts"][0]["text"], "USR");
        assert_eq!(sent["generationConfig"]["responseMimeType"], "application/json");
    }

    #[test]
    fn http_401_maps_to_auth() {
        let host = MockHost::new().with_http_response(401, b"nope");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(
            GeminiClient::new(&cfg()).complete(&ctx, &req()),
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
        match GeminiClient::new(&cfg()).complete(&ctx, &req()) {
            Err(ChatError::RateLimited { retry_after_ms }) => assert_eq!(retry_after_ms, Some(2000)),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn empty_content_is_malformed() {
        let host = MockHost::new().with_http_response(200, br#"{"candidates":[]}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(
            GeminiClient::new(&cfg()).complete(&ctx, &req()),
            Err(ChatError::Malformed(_))
        ));
    }

    #[test]
    fn http_500_maps_to_transport() {
        let host = MockHost::new().with_http_response(500, b"boom");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert!(matches!(
            GeminiClient::new(&cfg()).complete(&ctx, &req()),
            Err(ChatError::Transport(_))
        ));
    }
}
