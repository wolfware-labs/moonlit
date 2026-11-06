use std::path::{Path, PathBuf};

use crate::display::display_safe;
use crate::manifest::resolve_manifest_path;

pub(crate) const PERMISSION_REFUSAL: &str =
  "refusing to grant permissions without a terminal; pass --yes to accept the plugin's required permissions";

pub(crate) fn fail<T>(message: &str, code: i32) -> Result<T, i32> {
  eprintln!("error: {}", display_safe(message));
  Err(code)
}

/// Finds the release file named on the command line, or the default one, and reads it.
pub(crate) fn read_release(file_path: Option<&Path>) -> Result<(PathBuf, String), i32> {
  let path = resolve_manifest_path(file_path).or_else(|e| fail(&e.to_string(), 2))?;
  let content = std::fs::read_to_string(&path).or_else(|e| fail(&format!("cannot read '{}': {e}", path.display()), 2))?;
  Ok((path, content))
}

pub(crate) fn write_release(path: &Path, content: &str) -> Result<(), i32> {
  std::fs::write(path, content).or_else(|e| fail(&format!("cannot write '{}': {e}", path.display()), 2))
}

pub(crate) fn display_path(given: Option<&Path>, resolved: &Path) -> String {
  match given {
    Some(given) => given.display().to_string(),
    None => resolved
      .file_name()
      .map_or_else(|| resolved.display().to_string(), |n| n.to_string_lossy().into_owned()),
  }
}

pub(crate) fn prompt_failed<T>(e: &std::io::Error) -> Result<T, i32> {
  if e.kind() == std::io::ErrorKind::Interrupted {
    println!("Nothing changed.");
    Err(1)
  } else {
    fail(&e.to_string(), 2)
  }
}
