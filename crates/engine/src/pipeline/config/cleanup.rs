use crate::pipeline::config::model::PipelineConfig;

pub fn cleanup(mut config: PipelineConfig) -> PipelineConfig {
    config.name = config.name.trim().to_string();
    config
}
