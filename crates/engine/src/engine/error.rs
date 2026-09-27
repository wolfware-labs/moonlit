use crate::pipeline::PipelineError;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum EngineError {
  #[error(transparent)]
  #[diagnostic(transparent)]
  Pipeline(#[from] PipelineError),

  #[error("failed to load component {0}")]
  #[diagnostic(code(moonlit::engine::plugin_load))]
  ComponentLoad(String),

  #[error(transparent)]
  #[diagnostic(code(moonlit::engine::internal))]
  Internal(#[from] anyhow::Error),
}

impl EngineError {
  pub fn exit_code(&self) -> i32 {
    match self {
      EngineError::Pipeline(e) => e.exit_code(),
      EngineError::ComponentLoad(_) => 4,
      EngineError::Internal(_) => 1,
    }
  }
}
