//! Plugin-level config: the Slack API token, validated at `init`.

use moonlit_plugin_sdk::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SlackPluginConfig {
    pub token: String,
}

impl PluginConfig for SlackPluginConfig {
    fn validate(&self) -> Result<(), String> {
        if self.token.trim().is_empty() {
            return Err("Slack API token is required.".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_token_fails_with_exact_message() {
        let msg = match (SlackPluginConfig { token: "  ".into() }).validate() {
            Ok(()) => panic!("blank token must fail"),
            Err(e) => e,
        };
        assert_eq!(msg, "Slack API token is required.");
    }

    #[test]
    fn present_token_passes() {
        assert!((SlackPluginConfig { token: "xoxb-x".into() }).validate().is_ok());
    }
}
