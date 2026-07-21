//! Plugin-level config: the GitHub API token, validated at `init`.

use moonlit_plugin_sdk::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GithubPluginConfig {
    pub token: String,
}

impl PluginConfig for GithubPluginConfig {
    fn validate(&self) -> Result<(), String> {
        if self.token.trim().is_empty() {
            return Err("GitHub token is not configured.".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_token_fails_with_exact_message() {
        let msg = match (GithubPluginConfig { token: "  ".into() }).validate() {
            Ok(()) => panic!("blank token must fail"),
            Err(e) => e,
        };
        assert_eq!(msg, "GitHub token is not configured.");
    }

    #[test]
    fn present_token_passes() {
        assert!((GithubPluginConfig {
            token: "ghp_x".into()
        })
        .validate()
        .is_ok());
    }
}
