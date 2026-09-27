pub mod error;
mod model;

use crate::pipeline::manifest::error::PipelineManifestError;
pub use crate::pipeline::manifest::model::ManifestPeek;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct PipelineManifest {
  pub working_dir: PathBuf,
  pub file_name: String,
  pub content: String,
}

impl PipelineManifest {
  pub fn from_file(file_path: &Path) -> Result<PipelineManifest, PipelineManifestError> {
    if !file_path.is_file() {
      return Err(PipelineManifestError::new(format!(
        "Pipeline file '{}' does not exist.",
        file_path.display()
      )));
    }

    let working_dir = file_path.parent().ok_or(PipelineManifestError::new(format!(
      "No parent directory for pipeline file '{}'",
      file_path.display()
    )))?;

    let file_name = file_path.file_name().ok_or(PipelineManifestError::new(format!(
      "Error while getting the file name for {}",
      file_path.display()
    )))?;

    let content = std::fs::read_to_string(file_path)
      .map_err(|e| PipelineManifestError::new(format!("Error while reading {}: {e}", file_path.display())))?;

    Ok(PipelineManifest {
      working_dir: working_dir.to_path_buf(),
      file_name: file_name.to_string_lossy().to_string(),
      content,
    })
  }

  #[must_use]
  pub fn peek_name(&self) -> Option<String> {
    self.peek().and_then(|p| p.name).filter(|n| !n.trim().is_empty())
  }

  #[must_use]
  pub fn peek_stages(&self) -> Vec<String> {
    self
      .peek()
      .and_then(|p| p.stages)
      .map(|m| m.keys().cloned().collect())
      .unwrap_or_default()
  }

  #[must_use]
  fn peek(&self) -> Option<ManifestPeek> {
    serde_yaml_ng::from_str(&self.content).ok()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn manifest(content: &str) -> PipelineManifest {
    PipelineManifest {
      working_dir: PathBuf::new(),
      file_name: "release.yml".to_string(),
      content: content.to_string(),
    }
  }

  #[test]
  fn from_file_reads_the_file_and_its_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("release.yaml");
    std::fs::write(&path, "name: demo\n").unwrap();

    let manifest = PipelineManifest::from_file(&path).unwrap();
    assert_eq!(manifest.working_dir, dir.path());
    assert_eq!(manifest.file_name, "release.yaml");
    assert_eq!(manifest.content, "name: demo\n");
  }

  #[test]
  fn from_file_rejects_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("release.yml");
    let error = PipelineManifest::from_file(&path).unwrap_err();
    assert_eq!(
      error.to_string(),
      format!("Pipeline file '{}' does not exist.", path.display())
    );
    assert_eq!(error.exit_code(), 2);
  }

  #[test]
  fn from_file_rejects_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    let error = PipelineManifest::from_file(dir.path()).unwrap_err();
    assert!(error.to_string().ends_with("does not exist."));
  }

  #[test]
  fn from_file_rejects_non_utf8_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("release.yml");
    std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
    let error = PipelineManifest::from_file(&path).unwrap_err();
    assert!(error.to_string().starts_with("Error while reading "), "{error}");
  }

  #[test]
  fn peek_name_ignores_blank_names() {
    assert_eq!(manifest("name: demo\n").peek_name().as_deref(), Some("demo"));
    assert_eq!(manifest("name: '  '\n").peek_name(), None);
    assert_eq!(manifest("stages: {}\n").peek_name(), None);
  }

  #[test]
  fn peek_stages_lists_stage_names_in_order() {
    let peeked = manifest("stages:\n  build: []\n  deploy: []\n").peek_stages();
    assert_eq!(peeked, vec!["build", "deploy"]);
  }

  #[test]
  fn peeking_invalid_yaml_finds_nothing() {
    let broken = manifest("name: [unclosed\n");
    assert_eq!(broken.peek_name(), None);
    assert!(broken.peek_stages().is_empty());
  }
}
