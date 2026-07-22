//! Plugin-level config: the GitLab API token (validated at `init`) and base URL.

use moonlit_plugin_sdk::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GitlabPluginConfig {
    pub token: String,
    pub base_url: String,
}

impl Default for GitlabPluginConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            base_url: "https://gitlab.com".to_string(),
        }
    }
}

impl PluginConfig for GitlabPluginConfig {
    fn validate(&self) -> Result<(), String> {
        if self.token.trim().is_empty() {
            return Err("GitLab token is not configured.".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_token_fails_with_exact_message() {
        let msg = match (GitlabPluginConfig {
            token: "  ".into(),
            base_url: "https://gitlab.com".into(),
        })
        .validate()
        {
            Ok(()) => panic!("blank token must fail"),
            Err(e) => e,
        };
        assert_eq!(msg, "GitLab token is not configured.");
    }

    #[test]
    fn present_token_passes() {
        assert!((GitlabPluginConfig {
            token: "glpat_x".into(),
            base_url: "https://gitlab.com".into()
        })
        .validate()
        .is_ok());
    }

    #[test]
    fn default_base_url_is_gitlab_com() {
        assert_eq!(GitlabPluginConfig::default().base_url, "https://gitlab.com");
        assert_eq!(GitlabPluginConfig::default().token, "");
    }

    #[test]
    fn missing_base_url_falls_back_to_default() {
        let c: GitlabPluginConfig = serde_json::from_str(r#"{"token":"t"}"#).unwrap();
        assert_eq!(c.base_url, "https://gitlab.com");
    }

    #[test]
    fn base_url_override_is_read() {
        let c: GitlabPluginConfig =
            serde_json::from_str(r#"{"token":"t","baseUrl":"https://gl.example.com"}"#).unwrap();
        assert_eq!(c.base_url, "https://gl.example.com");
    }
}
