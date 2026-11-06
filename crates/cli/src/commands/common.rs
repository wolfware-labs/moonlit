use std::path::Path;

use crate::display::display_safe;

pub(crate) const PERMISSION_REFUSAL: &str =
  "refusing to grant permissions without a terminal; pass --yes to accept the plugin's required permissions";

pub(crate) fn fail<T>(message: &str, code: i32) -> Result<T, i32> {
  eprintln!("error: {}", display_safe(message));
  Err(code)
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
