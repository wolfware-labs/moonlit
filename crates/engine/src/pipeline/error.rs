use crate::pipeline::config::ConfigDiagnostic;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncompatiblePlugin {
  pub name: String,
  pub url: String,
  pub plugin_world: Option<String>,
  pub engine_world: String,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PipelineError {
  #[error(transparent)]
  #[diagnostic(transparent)]
  Config(#[from] ConfigDiagnostic),

  #[error("failed to load plugin '{plugin}': {message}")]
  #[diagnostic(code(moonlit::pipeline::plugin_load))]
  PluginLoad { plugin: String, message: String },

  #[error("{} plugin(s) target a different plugin contract than this CLI ({})", .0.len(), .0.first().map_or("", |p| p.engine_world.as_str()))]
  #[diagnostic(code(moonlit::pipeline::incompatible_plugins))]
  IncompatiblePlugins(Vec<IncompatiblePlugin>),

  #[error("pipeline execution failed: {0}")]
  #[diagnostic(code(moonlit::pipeline::execution))]
  Execution(String),

  #[error(transparent)]
  #[diagnostic(code(moonlit::pipeline::internal))]
  Internal(#[from] anyhow::Error),
}

impl PipelineError {
  #[must_use]
  pub fn exit_code(&self) -> i32 {
    match self {
      PipelineError::Config(_) => 2,
      PipelineError::PluginLoad { .. } | PipelineError::IncompatiblePlugins(_) => 3,
      PipelineError::Execution(_) => 4,
      PipelineError::Internal(_) => 1,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pipeline::config::parse_config;

  #[test]
  fn every_variant_maps_to_its_exit_code() {
    let config = PipelineError::from(parse_config("- a\n", "release.yml").unwrap_err());
    let load = PipelineError::PluginLoad {
      plugin: "p".to_string(),
      message: "m".to_string(),
    };
    let execution = PipelineError::Execution("x".to_string());
    let internal = PipelineError::from(anyhow::anyhow!("boom"));

    assert_eq!(config.exit_code(), 2);
    assert_eq!(load.exit_code(), 3);
    assert_eq!(incompatible_plugins().exit_code(), 3);
    assert_eq!(execution.exit_code(), 4);
    assert_eq!(internal.exit_code(), 1);
  }

  fn incompatible_plugins() -> PipelineError {
    let plugin = |name: &str| IncompatiblePlugin {
      name: name.to_string(),
      url: format!("file:///{name}.wasm"),
      plugin_world: Some("moonlit:plugin@0.3.0".to_string()),
      engine_world: "moonlit:plugin@0.4.0".to_string(),
    };
    PipelineError::IncompatiblePlugins(vec![plugin("a"), plugin("b")])
  }

  #[test]
  fn incompatible_plugins_message_counts_plugins_and_names_the_engine_world() {
    assert_eq!(
      incompatible_plugins().to_string(),
      "2 plugin(s) target a different plugin contract than this CLI (moonlit:plugin@0.4.0)"
    );
  }

  #[test]
  fn messages_name_the_failing_piece() {
    let load = PipelineError::PluginLoad {
      plugin: "git".to_string(),
      message: "not found".to_string(),
    };
    assert_eq!(load.to_string(), "failed to load plugin 'git': not found");
    assert_eq!(
      PipelineError::Execution("halted".to_string()).to_string(),
      "pipeline execution failed: halted"
    );
  }
}
