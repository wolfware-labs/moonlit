mod cleanup;
mod convert;
mod diagnostic;
mod model;
mod tree;
mod validate;

pub use crate::pipeline::config::diagnostic::ConfigDiagnostic;
use crate::pipeline::config::diagnostic::Source;
pub use crate::pipeline::config::model::PipelineConfig;

pub fn parse_config(yaml: &str, source_name: &str) -> Result<PipelineConfig, ConfigDiagnostic> {
    let src = Source::new(yaml, source_name);
    let tree = tree::build_tree(&src)?;
    let config = convert::convert(tree, &src)?;
    let config = cleanup::cleanup(config);
    validate::validate(&config, &src)?;
    Ok(config)
}
