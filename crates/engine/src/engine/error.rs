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
  #[must_use]
  pub fn exit_code(&self) -> i32 {
    match self {
      EngineError::Pipeline(e) => e.exit_code(),
      EngineError::ComponentLoad(_) => 3,
      EngineError::Internal(_) => 1,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn component_load_failures_exit_like_plugin_load_failures() {
    assert_eq!(EngineError::ComponentLoad("bad".to_string()).exit_code(), 3);
  }
}
