use anyhow::anyhow;
use std::path::PathBuf;

const DEFAULT_NAMES: [&str; 2] = ["release.yml", "release.yaml"];

pub fn resolve_manifest_path(
    working_dir: Option<PathBuf>,
    file_name: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let working_dir = working_dir.unwrap_or_else(|| PathBuf::from("."));
    if !working_dir.is_dir() {
        return Err(anyhow!(
            "Working directory '{}' does not exist.",
            working_dir.display()
        ));
    }

    let working_dir = working_dir
        .canonicalize()
        .map_err(|e| anyhow!("resolving working directory: {e}"))?;

    let Some(file_name) = file_name else {
        return DEFAULT_NAMES
            .iter()
            .map(|n| working_dir.join(n))
            .find(|p| p.is_file())
            .ok_or(anyhow!(
                "No pipeline file found in '{}' (looked for {}).",
                working_dir.display(),
                DEFAULT_NAMES.join(", ")
            ));
    };

    let ext_ok = file_name
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("yml") || e.eq_ignore_ascii_case("yaml"))
        .unwrap_or(false);
    if !ext_ok {
        return Err(anyhow!(
            "Pipeline file '{}' must have a .yml or .yaml extension.",
            file_name
        ));
    }
    let path = working_dir.join(file_name);
    if !path.is_file() {
        return Err(anyhow!(
            "Pipeline file '{}' does not exist.",
            path.display()
        ));
    }
    Ok(path)
}
