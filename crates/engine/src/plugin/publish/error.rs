#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PublishError {
  #[error("invalid plugin reference: {0}")]
  #[diagnostic(code(moonlit::publish::invalid_reference))]
  InvalidReference(String),
  #[error("authentication failed for registry: {0}")]
  #[diagnostic(code(moonlit::publish::auth))]
  Auth(String),
  #[error("registry error while publishing: {0}")]
  #[diagnostic(code(moonlit::publish::network))]
  Network(String),
  #[error("publish I/O error: {0}")]
  #[diagnostic(code(moonlit::publish::io))]
  Io(String),
}
