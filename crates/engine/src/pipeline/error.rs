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
  pub fn exit_code(&self) -> i32 {
    match self {
      PipelineError::Config(_) => 2,
      PipelineError::PluginLoad { .. } => 3,
      PipelineError::Execution(_) => 5,
      PipelineError::Internal(_) => 1,
    }
  }
}
