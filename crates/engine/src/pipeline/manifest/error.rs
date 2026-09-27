#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{0}")]
#[diagnostic(code(moonlit::pipeline::manifest))]
pub struct PipelineManifestError(String);

impl PipelineManifestError {
  #[must_use]
  pub fn new(message: String) -> Self {
    Self(message)
  }
  #[must_use]
  pub fn exit_code(&self) -> i32 {
    2
  }
}
