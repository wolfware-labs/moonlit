//! Plugin-level config: NuGet source + API key, with an `apiKey` alias for `nugetApiKey`.

use moonlit_sdk::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DotnetConfig {
    pub nuget_source: String,
    pub nuget_api_key: String,
    pub api_key: String,
}

impl Default for DotnetConfig {
    fn default() -> Self {
        Self {
            nuget_source: "https://api.nuget.org/v3/index.json".to_string(),
            nuget_api_key: String::new(),
            api_key: String::new(),
        }
    }
}

impl DotnetConfig {
    /// Resolved NuGet API key: prefer `nugetApiKey`, fall back to the `apiKey` alias.
    pub fn resolved_api_key(&self) -> &str {
        if !self.nuget_api_key.trim().is_empty() {
            &self.nuget_api_key
        } else {
            &self.api_key
        }
    }
}

impl PluginConfig for DotnetConfig {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_nuget_org_and_blank_keys() {
        let c = DotnetConfig::default();
        assert_eq!(c.nuget_source, "https://api.nuget.org/v3/index.json");
        assert_eq!(c.resolved_api_key(), "");
    }

    #[test]
    fn nuget_api_key_preferred_over_alias() {
        let c: DotnetConfig =
            serde_json::from_value(serde_json::json!({ "nugetApiKey": "A", "apiKey": "B" }))
                .unwrap();
        assert_eq!(c.resolved_api_key(), "A");
    }

    #[test]
    fn alias_used_when_nuget_api_key_blank() {
        let c: DotnetConfig =
            serde_json::from_value(serde_json::json!({ "nugetApiKey": "  ", "apiKey": "B" }))
                .unwrap();
        assert_eq!(c.resolved_api_key(), "B");
    }

    #[test]
    fn missing_source_falls_back_to_default() {
        let c: DotnetConfig = serde_json::from_value(serde_json::json!({ "apiKey": "B" })).unwrap();
        assert_eq!(c.nuget_source, "https://api.nuget.org/v3/index.json");
    }
}
