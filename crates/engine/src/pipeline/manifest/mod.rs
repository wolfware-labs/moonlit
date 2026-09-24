pub mod error;
mod model;

use crate::pipeline::manifest::error::PipelineManifestError;
pub use crate::pipeline::manifest::model::ManifestPeek;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PipelineManifest {
    pub working_dir: PathBuf,
    pub file_name: String,
    pub content: String,
}

impl PipelineManifest {
    pub fn from_file(file_path: PathBuf) -> Result<PipelineManifest, PipelineManifestError> {
        if !file_path.is_file() {
            return Err(PipelineManifestError::new(format!(
                "Pipeline file '{}' does not exist.",
                file_path.display()
            )));
        }

        let working_dir = file_path
            .parent()
            .ok_or(PipelineManifestError::new(format!(
                "No parent directory for pipeline file '{}'",
                file_path.display()
            )))?;

        let file_name = file_path
            .file_name()
            .ok_or(PipelineManifestError::new(format!(
                "Error while getting the file name for {}",
                file_path.display()
            )))?;

        let content = std::fs::read_to_string(&file_path).map_err(|e| {
            PipelineManifestError::new(format!("Error while reading {}: {e}", file_path.display()))
        })?;

        Ok(PipelineManifest {
            working_dir: working_dir.to_path_buf(),
            file_name: file_name.to_string_lossy().to_string(),
            content,
        })
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
