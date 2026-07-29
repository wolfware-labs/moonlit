//! Plugin-level config: npm registry + auth token (fallbacks for `push`).

use moonlit_sdk::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeConfig {
    pub registry: String,
    pub token: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            registry: "https://registry.npmjs.org".to_string(),
            token: String::new(),
        }
    }
}

impl PluginConfig for NodeConfig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_npmjs_and_blank_token() {
        let c = NodeConfig::default();
        assert_eq!(c.registry, "https://registry.npmjs.org");
        assert_eq!(c.token, "");
    }

    #[test]
    fn missing_registry_falls_back_to_default() {
        let c: NodeConfig = serde_json::from_value(serde_json::json!({ "token": "T" })).unwrap();
        assert_eq!(c.registry, "https://registry.npmjs.org");
        assert_eq!(c.token, "T");
    }

    #[test]
    fn explicit_registry_and_token_parsed() {
        let c: NodeConfig =
            serde_json::from_value(serde_json::json!({ "registry": "https://r", "token": "T" }))
                .unwrap();
        assert_eq!(c.registry, "https://r");
        assert_eq!(c.token, "T");
    }
}
