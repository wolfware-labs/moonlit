//! Provider-agnostic chat abstraction (Microsoft.Extensions.AI-style). The trait
//! fronts every LLM provider; `Retrying` adds backoff; `build_client` picks the impl.

use moonlit_plugin_sdk::prelude::*;

// NOTE: `pub mod openai;` is added in Task 4 (openai.rs does not exist yet).

/// A single-turn chat request, normalized across providers.
pub struct ChatRequest {
    pub system: String,
    pub user: String,
}

/// The model's raw text answer, with any ``` code fences already stripped.
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

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::config::from_json_value;

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
