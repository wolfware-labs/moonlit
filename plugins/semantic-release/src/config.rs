//! Plugin-level config for semantic-release: an optional `ai` block that powers
//! generate-changelog's AI refinement. Absent by default (most pipelines don't use AI).

use moonlit_plugin_sdk::prelude::*;

use crate::ai::AiConfig;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SrPluginConfig {
    pub ai: Option<AiConfig>,
}

impl PluginConfig for SrPluginConfig {
    fn validate(&self) -> Result<(), String> {
        if let Some(ai) = &self.ai {
            if ai.api_key.trim().is_empty() {
                return Err("The 'ai' config block requires a non-empty apiKey.".to_string());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::config::from_json_value;

    #[test]
    fn absent_ai_is_valid_and_default() {
        let c: SrPluginConfig = from_json_value("{}").unwrap();
        assert!(c.ai.is_none());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn ai_present_with_key_is_valid() {
        let c: SrPluginConfig = from_json_value(r#"{"ai":{"apiKey":"sk-x"}}"#).unwrap();
        assert!(c.ai.is_some());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn ai_present_blank_key_fails() {
        let c: SrPluginConfig = from_json_value(r#"{"ai":{"apiKey":"  "}}"#).unwrap();
        assert_eq!(
            c.validate().unwrap_err(),
            "The 'ai' config block requires a non-empty apiKey."
        );
    }
}
