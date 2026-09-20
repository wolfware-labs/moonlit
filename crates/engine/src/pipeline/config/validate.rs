use crate::pipeline::config::diagnostic::{ConfigDiagnostic, Source};
use crate::pipeline::config::model::PipelineConfig;

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
