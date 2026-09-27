use std::ffi::OsString;
use std::path::PathBuf;

pub const HOME_OVERRIDE: &str = "MOONLIT_HOME";
pub const CACHE_OVERRIDE: &str = "MOONLIT_CACHE_DIR";

#[must_use]
pub fn home_dir() -> Option<PathBuf> {
  resolve(std::env::var_os(HOME_OVERRIDE), dirs::home_dir())
}

#[must_use]
pub fn cache_dir() -> Option<PathBuf> {
  resolve(
    std::env::var_os(CACHE_OVERRIDE),
    dirs::cache_dir().map(|dir| dir.join("moonlit")),
  )
}

#[must_use]
fn resolve(override_value: Option<OsString>, fallback: Option<PathBuf>) -> Option<PathBuf> {
  override_value
    .filter(|value| !value.is_empty())
    .map(PathBuf::from)
    .or(fallback)
    .filter(|path| !path.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn override_wins_over_fallback() {
    let resolved = resolve(Some(OsString::from("/override")), Some(PathBuf::from("/fallback")));
    assert_eq!(resolved, Some(PathBuf::from("/override")));
  }

  #[test]
  fn fallback_is_used_without_override() {
    assert_eq!(
      resolve(None, Some(PathBuf::from("/fallback"))),
      Some(PathBuf::from("/fallback"))
    );
  }

  #[test]
  fn empty_override_falls_back() {
    assert_eq!(
      resolve(Some(OsString::new()), Some(PathBuf::from("/fallback"))),
      Some(PathBuf::from("/fallback"))
    );
  }

  #[test]
  fn empty_fallback_resolves_to_none() {
    assert_eq!(resolve(None, Some(PathBuf::new())), None);
  }

  #[test]
  fn nothing_resolves_to_none() {
    assert_eq!(resolve(None, None), None);
  }
}
