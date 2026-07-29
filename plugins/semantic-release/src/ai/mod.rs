//! Provider-agnostic chat abstraction (Microsoft.Extensions.AI-style). The trait
//! fronts every LLM provider; `Retrying` adds backoff; `build_client` picks the impl.

use moonlit_plugin_sdk::prelude::*;

pub mod openai;

/// A single-turn chat request, normalized across providers.
pub struct ChatRequest {
    pub system: String,
    pub user: String,
}

/// The model's raw text answer, with any ``` code fences already stripped.
#[derive(Debug)]
pub struct ChatResponse {
    pub text: String,
}

/// Provider error, normalized so the retry policy is provider-agnostic.
#[derive(Debug)]
pub enum ChatError {
    RateLimited { retry_after_ms: Option<u64> },
    Transport(String),
    Auth(String),
    Malformed(String),
}

pub trait ChatClient {
    fn complete(&self, ctx: &Context, req: &ChatRequest) -> Result<ChatResponse, ChatError>;
}

fn default_max_retries() -> u32 {
    5
}

#[derive(Deserialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Openai,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct AiConfig {
    pub provider: Provider,
    pub model: Option<String>,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_retries: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: Provider::Openai,
            model: None,
            api_key: String::new(),
            base_url: None,
            max_retries: default_max_retries(),
        }
    }
}

impl AiConfig {
    /// The model to use: explicit `model`, else the provider's default.
    pub fn model_or_default(&self) -> &str {
        match &self.model {
            Some(m) if !m.trim().is_empty() => m,
            _ => match self.provider {
                Provider::Openai => "gpt-5-mini",
            },
        }
    }
}

/// Backoff in ms for a failed attempt: honor `retry_after_ms` if present, else
/// `2^attempt × 500ms`; capped at 60_000ms.
fn backoff_ms(attempt: u32, retry_after_ms: Option<u64>) -> u64 {
    let ms = match retry_after_ms {
        Some(ra) => ra,
        None => 500u64.saturating_mul(1u64.checked_shl(attempt).unwrap_or(u64::MAX)),
    };
    ms.min(60_000)
}

fn retry_after_of(e: &ChatError) -> Option<u64> {
    match e {
        ChatError::RateLimited { retry_after_ms } => *retry_after_ms,
        _ => None,
    }
}

/// Decorator: retries `RateLimited`/`Transport` with backoff; returns
/// `Auth`/`Malformed` immediately. Waits via the SDK monotonic sleep.
pub struct Retrying {
    inner: Box<dyn ChatClient>,
    max_retries: u32,
}

impl Retrying {
    pub fn new(inner: Box<dyn ChatClient>, max_retries: u32) -> Self {
        Self { inner, max_retries }
    }
}

impl ChatClient for Retrying {
    fn complete(&self, ctx: &Context, req: &ChatRequest) -> Result<ChatResponse, ChatError> {
        let mut attempt = 0u32;
        loop {
            match self.inner.complete(ctx, req) {
                Ok(r) => return Ok(r),
                Err(e @ (ChatError::Auth(_) | ChatError::Malformed(_))) => return Err(e),
                Err(e) => {
                    if attempt >= self.max_retries {
                        return Err(e);
                    }
                    ctx.clock().sleep_ms(backoff_ms(attempt, retry_after_of(&e)));
                    attempt += 1;
                }
            }
        }
    }
}

/// Build a ready-to-use client from config: the provider impl wrapped in `Retrying`.
pub fn build_client(cfg: &AiConfig) -> Box<dyn ChatClient> {
    let base: Box<dyn ChatClient> = match cfg.provider {
        Provider::Openai => Box::new(openai::OpenAiClient::new(cfg)),
    };
    Box::new(Retrying::new(base, cfg.max_retries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::config::from_json_value;
    use moonlit_plugin_sdk::testing::MockHost;
    use std::cell::RefCell;

    /// Fake client returning a scripted sequence of results, one per call.
    struct FakeClient {
        results: RefCell<Vec<Result<ChatResponse, ChatError>>>,
        calls: RefCell<usize>,
    }
    impl FakeClient {
        fn new(results: Vec<Result<ChatResponse, ChatError>>) -> Self {
            Self { results: RefCell::new(results), calls: RefCell::new(0) }
        }
    }
    impl ChatClient for FakeClient {
        fn complete(&self, _ctx: &Context, _req: &ChatRequest) -> Result<ChatResponse, ChatError> {
            *self.calls.borrow_mut() += 1;
            let mut r = self.results.borrow_mut();
            if r.is_empty() { return Err(ChatError::Transport("exhausted".into())); }
            r.remove(0)
        }
    }
    fn req() -> ChatRequest { ChatRequest { system: "s".into(), user: "u".into() } }

    #[test]
    fn backoff_uses_retry_after_then_exponential() {
        assert_eq!(backoff_ms(0, None), 500);
        assert_eq!(backoff_ms(1, None), 1000);
        assert_eq!(backoff_ms(4, None), 8000);
        assert_eq!(backoff_ms(0, Some(1000)), 1000);   // honor server hint
        assert_eq!(backoff_ms(3, Some(90_000)), 60_000); // capped
    }

    #[test]
    fn success_passes_through_without_sleep() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let r = Retrying::new(Box::new(FakeClient::new(vec![Ok(ChatResponse { text: "ok".into() })])), 5);
        assert_eq!(r.complete(&ctx, &req()).unwrap().text, "ok");
        assert!(host.recorded_sleeps().is_empty());
    }

    #[test]
    fn auth_error_is_not_retried() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let fake = Box::new(FakeClient::new(vec![Err(ChatError::Auth("401".into()))]));
        let r = Retrying::new(fake, 5);
        assert!(matches!(r.complete(&ctx, &req()), Err(ChatError::Auth(_))));
        assert!(host.recorded_sleeps().is_empty());
    }

    #[test]
    fn rate_limited_then_success_sleeps_once() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let fake = Box::new(FakeClient::new(vec![
            Err(ChatError::RateLimited { retry_after_ms: Some(1000) }),
            Ok(ChatResponse { text: "done".into() }),
        ]));
        let r = Retrying::new(fake, 5);
        assert_eq!(r.complete(&ctx, &req()).unwrap().text, "done");
        assert_eq!(host.recorded_sleeps(), vec![1_000_000_000]); // 1000ms -> 1e9 nanos
    }

    #[test]
    fn exhausts_retries_and_fails_with_backoff_sequence() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let fake = Box::new(FakeClient::new(vec![
            Err(ChatError::RateLimited { retry_after_ms: None }), // attempt 0
            Err(ChatError::RateLimited { retry_after_ms: None }), // 1
            Err(ChatError::RateLimited { retry_after_ms: None }), // 2
            Err(ChatError::RateLimited { retry_after_ms: None }), // 3
            Err(ChatError::RateLimited { retry_after_ms: None }), // 4
            Err(ChatError::RateLimited { retry_after_ms: None }), // 5 (== max_retries) -> return Err
        ]));
        let r = Retrying::new(fake, 5);
        assert!(matches!(r.complete(&ctx, &req()), Err(ChatError::RateLimited { .. })));
        // 5 sleeps between the 6 attempts, exponential in ms -> nanos.
        assert_eq!(host.recorded_sleeps(),
            vec![500_000_000, 1_000_000_000, 2_000_000_000, 4_000_000_000, 8_000_000_000]);
    }

    #[test]
    fn build_client_wraps_openai_in_retry() {
        // 429 (retry-after 1s) then 200 -> factory-built client retries and succeeds.
        let host = MockHost::new()
            .with_http_response_headers(429, vec![("retry-after".into(), "1".into())], b"slow")
            .with_http_response(200, br#"{"choices":[{"message":{"content":"{\"ok\":true}"}}]}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let cfg: AiConfig = from_json_value(r#"{"apiKey":"sk"}"#).unwrap();
        let client = build_client(&cfg);
        let out = client.complete(&ctx, &ChatRequest { system: "s".into(), user: "u".into() }).unwrap();
        assert_eq!(out.text, "{\"ok\":true}");
        assert_eq!(host.recorded_sleeps(), vec![1_000_000_000]); // retried once; 1000ms=1e9 nanos
    }

    #[test]
    fn ai_config_defaults_and_camel_case() {
        let c: AiConfig = from_json_value(r#"{"apiKey":"sk-x"}"#).unwrap();
        assert!(matches!(c.provider, Provider::Openai));
        assert_eq!(c.api_key, "sk-x");
        assert_eq!(c.max_retries, 5);
        assert_eq!(c.model_or_default(), "gpt-5-mini");
        assert!(c.base_url.is_none());
    }

    #[test]
    fn ai_config_explicit_model_wins() {
        let c: AiConfig = from_json_value(r#"{"apiKey":"k","model":"gpt-4o","maxRetries":2}"#).unwrap();
        assert_eq!(c.model_or_default(), "gpt-4o");
        assert_eq!(c.max_retries, 2);
    }
}
