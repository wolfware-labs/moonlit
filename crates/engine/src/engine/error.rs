use crate::pipeline::config::ConfigDiagnostic;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum EngineError {
  #[error(transparent)]
  #[diagnostic(transparent)]
  Config(#[from] ConfigDiagnostic),

  #[error("failed to load plugin '{plugin}': {message}")]
  #[diagnostic(code(moonlit::engine::plugin_load))]
  PluginLoad { plugin: String, message: String },

  #[error("failed to load component {0}")]
  #[diagnostic(code(moonlit::engine::plugin_load))]
  ComponentLoad(String),

  #[error("pipeline execution failed: {0}")]
  #[diagnostic(code(moonlit::engine::execution))]
  Execution(String),

  #[error(transparent)]
  #[diagnostic(code(moonlit::engine::internal))]
  Internal(#[from] anyhow::Error),
}

impl EngineError {
  pub fn exit_code(&self) -> i32 {
    match self {
      EngineError::Config(_) => 2,
      EngineError::PluginLoad { .. } => 3,
      EngineError::ComponentLoad(_) => 4,
      EngineError::Execution(_) => 5,
      EngineError::Internal(_) => 1,
    }
  }
}
