#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{0}")]
#[diagnostic(code(moonlit::pipeline::manifest))]
pub struct PipelineManifestError(String);

impl PipelineManifestError {
    pub fn new(message: String) -> Self {
        Self(message)
    }
    pub fn exit_code(&self) -> i32 {
        2
    }
}
