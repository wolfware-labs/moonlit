use moonlit_engine::pipeline::manifest::error::PipelineManifestError;
use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_NAMES: [&str; 2] = ["release.yml", "release.yaml"];

pub fn resolve_manifest_path(file_name: Option<&Path>) -> Result<PathBuf, PipelineManifestError> {
    let working_dir = env::current_dir().map_err(|e| PipelineManifestError::new(e.to_string()))?;
    let Some(file_name) = file_name else {
        return DEFAULT_NAMES
            .iter()
            .map(|n| working_dir.join(n))
            .find(|p| p.is_file())
            .ok_or(PipelineManifestError::new(format!(
                "No pipeline file found in '{}' (looked for {}).",
                working_dir.display(),
                DEFAULT_NAMES.join(", ")
            )));
    };

    let path = working_dir.join(file_name);
    if !path.is_file() {
        return Err(PipelineManifestError::new(format!(
            "Pipeline file '{}' does not exist.",
            path.display()
        )));
    }
    Ok(path)
}
