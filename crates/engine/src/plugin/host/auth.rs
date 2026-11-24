use crate::pipeline::config::FilesystemAccess;
use crate::plugin::instance::PluginInstanceConfig;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder};

/// `pattern` with each run of `*` folded into one, so `**` means what `*` does and crosses no path separator.
fn single_stars(pattern: &str) -> String {
  let mut out = String::with_capacity(pattern.len());
  for c in pattern.chars() {
    if c != '*' || !out.ends_with('*') {
      out.push(c);
    }
  }
  out
}

/// The patterns of a grant as one matcher. A wildcard, `**` included, never matches a path separator, so a grant
/// for `git*` does not cover a program path such as `git/../sh`.
#[must_use]
pub fn build_globset(patterns: &[String]) -> GlobSet {
  let mut b = GlobSetBuilder::new();
  for p in patterns {
    if let Ok(g) = GlobBuilder::new(&single_stars(p)).literal_separator(true).build() {
      b.add(g);
    }
  }
  b.build().unwrap_or_else(|_| GlobSet::empty())
}

#[must_use]
pub fn filter_env(patterns: &[String], snapshot: &[(String, String)]) -> Vec<(String, String)> {
  let gs = build_globset(patterns);
  snapshot.iter().filter(|(k, _)| gs.is_match(k)).cloned().collect()
}

#[must_use]
pub fn filesystem_perms(access: FilesystemAccess) -> Option<FsPerms> {
  match access {
    FilesystemAccess::None => None,
    FilesystemAccess::ReadOnly => Some(FsPerms::ReadOnly),
    FilesystemAccess::ReadWrite => Some(FsPerms::ReadWrite),
  }
}

pub fn build_wasi_ctx(cfg: &PluginInstanceConfig) -> anyhow::Result<WasiCtx> {
  let mut b = WasiCtxBuilder::new();
  for (k, v) in filter_env(&cfg.permissions.env, &cfg.env_snapshot) {
    b.env(&k, &v);
  }
  if let Some(perms) = filesystem_perms(cfg.permissions.filesystem) {
    b.preopened_dir(&cfg.working_directory, ".", perms)?;
  }
  Ok(b.build())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn matches(pattern: &str, value: &str) -> bool {
    build_globset(&[pattern.to_string()]).is_match(value)
  }

  #[test]
  fn no_wildcard_matches_a_path_separator() {
    for pattern in ["*", "**", "***", "/usr/**", "**/git", "g?t"] {
      for value in ["/bin/sh", "git/sh", "/usr/bin/git", "/tmp/evil/git", "g/t"] {
        assert!(!matches(pattern, value), "{pattern} {value}");
      }
    }
    assert!(matches("/usr/*/git", "/usr/bin/git"));
    assert!(matches("/usr/**/git", "/usr/bin/git"));
    assert!(matches("**", "git"));
    assert!(matches("*.example.com", "a.b.example.com"));
    assert!(matches("GIT_**", "GIT_AUTHOR_NAME"));
  }

  #[test]
  fn filesystem_access_maps_to_wasi_permissions() {
    assert!(filesystem_perms(FilesystemAccess::None).is_none());
    assert!(matches!(
      filesystem_perms(FilesystemAccess::ReadOnly),
      Some(FsPerms::ReadOnly)
    ));
    assert!(matches!(
      filesystem_perms(FilesystemAccess::ReadWrite),
      Some(FsPerms::ReadWrite)
    ));
  }
}
