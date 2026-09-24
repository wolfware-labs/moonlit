pub mod error;
mod model;

use crate::pipeline::manifest::error::PipelineManifestError;
pub use crate::pipeline::manifest::model::ManifestPeek;
use std::path::PathBuf;

pub struct PipelineManifest {
    path: PathBuf,
    content: String,
}

impl PipelineManifest {
    pub fn from_file(file_path: PathBuf) -> Result<PipelineManifest, PipelineManifestError> {
        if !file_path.is_file() {
            return Err(PipelineManifestError::new(format!(
                "Pipeline file '{}' does not exist.",
                file_path.display()
            )));
        }

        let content = std::fs::read_to_string(&file_path).map_err(|e| {
            PipelineManifestError::new(format!("reading {}: {e}", file_path.display()))
        })?;

        Ok(PipelineManifest {
            path: file_path,
            content,
        })
    }

    pub fn file_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
    }

    pub fn peek_name(&self) -> Option<String> {
        self.peek()
            .and_then(|p| p.name)
            .filter(|n| !n.trim().is_empty())
    }

    pub fn peek_stages(&self) -> Vec<String> {
        self.peek()
            .and_then(|p| p.stages)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn peek(&self) -> Option<ManifestPeek> {
        serde_yaml_ng::from_str(&self.content).ok()
    }
}
