use crate::pipeline::config::ConfigDiagnostic;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PipelineError {
  #[error(transparent)]
  #[diagnostic(transparent)]
  Config(#[from] ConfigDiagnostic),

  #[error("failed to load plugin '{plugin}': {message}")]
  #[diagnostic(code(moonlit::pipeline::plugin_load))]
  PluginLoad { plugin: String, message: String },

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
      PipelineError::PluginLoad { .. } => 3,
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
    assert_eq!(execution.exit_code(), 4);
    assert_eq!(internal.exit_code(), 1);
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
