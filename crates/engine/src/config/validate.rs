//! Structural/semantic validation. Exit-code-2 class validation errors. `run`
//! format is validated in `convert` (where the raw value and span live); this stage handles only
//! the cross-cutting rules that need the whole model.

use crate::config::diagnostic::{ConfigDiagnostic, Source};
use crate::config::model::PipelineConfig;

/// Validate a cleaned pipeline config. Checks stages-first, then plugins (§4.2 order).
pub fn validate(config: &PipelineConfig, src: &Source) -> Result<(), ConfigDiagnostic> {
    if config.stages.value.is_empty() {
        return Err(src.no_stages());
    }
    if config.plugins.value.is_empty() {
        return Err(src.no_plugins(Some(config.plugins.span)));
    }
    let mut seen = std::collections::HashSet::new();
    for plugin in &config.plugins.value {
        if !seen.insert(plugin.name.as_str()) {
            return Err(src.duplicate_plugin(&plugin.name, config.plugins.span));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::diagnostic::Source;
    use crate::config::model::{PipelineConfig, Plugin, PluginUrl, Span, Spanned, Stage};
    use indexmap::IndexMap;

    fn config(plugins: Vec<Plugin>, stages: Vec<Stage>) -> PipelineConfig {
        PipelineConfig {
            name: String::new(),
            arguments: IndexMap::new(),
            variables: IndexMap::new(),
            plugins: Spanned::new(plugins, Span::point(0)),
            stages: Spanned::new(stages, Span::point(0)),
        }
    }

    fn a_plugin() -> Plugin {
        Plugin {
            name: "p".to_string(),
            url: Spanned::new(
                PluginUrl::File("file:///p.wasm".to_string()),
                Span::point(0),
            ),
            config: IndexMap::new(),
            permissions: None,
        }
    }

    fn a_stage() -> Stage {
        Stage {
            name: "s".to_string(),
            steps: Vec::new(),
        }
    }

    fn named_plugin(name: &str) -> Plugin {
        Plugin {
            name: name.to_string(),
            url: Spanned::new(
                PluginUrl::File("file:///p.wasm".to_string()),
                Span::point(0),
            ),
            config: IndexMap::new(),
            permissions: None,
        }
    }

    #[test]
    fn zero_stages_is_an_error() {
        let c = config(vec![a_plugin()], Vec::new());
        let err = validate(&c, &Source::new("", "release.yml")).unwrap_err();
        assert_eq!(
            err.message(),
            "No stages defined. A pipeline needs at least one stage."
        );
    }

    #[test]
    fn stages_but_no_plugins_is_an_error() {
        let c = config(Vec::new(), vec![a_stage()]);
        let err = validate(&c, &Source::new("", "release.yml")).unwrap_err();
        assert_eq!(
            err.message(),
            "No plugins declared. Every step runs a middleware from a plugin, so at least one is required."
        );
    }

    #[test]
    fn valid_config_passes() {
        let c = config(vec![a_plugin()], vec![a_stage()]);
        assert!(validate(&c, &Source::new("", "release.yml")).is_ok());
    }

    #[test]
    fn duplicate_plugin_names_are_rejected() {
        let c = config(
            vec![named_plugin("tp"), named_plugin("tp")],
            vec![a_stage()],
        );
        let err = validate(&c, &Source::new("", "release.yml")).unwrap_err();
        assert_eq!(
            err.message(),
            "Duplicate plugin name 'tp'. Plugin names must be unique."
        );
    }
}
