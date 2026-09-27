use crate::pipeline::config::FilesystemAccess;
use crate::plugin::instance::PluginInstanceConfig;
use globset::{Glob, GlobSet, GlobSetBuilder};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder};

#[must_use]
fn build_globset(patterns: &[String]) -> GlobSet {
  let mut b = GlobSetBuilder::new();
  for p in patterns {
    if let Ok(g) = Glob::new(p) {
      b.add(g);
    }
  }
  b.build().unwrap_or_else(|_| GlobSet::empty())
}

#[must_use]
pub fn exec_globset(patterns: &[String]) -> GlobSet {
  build_globset(patterns)
}

#[must_use]
pub fn network_globset(patterns: &[String]) -> GlobSet {
  build_globset(patterns)
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
