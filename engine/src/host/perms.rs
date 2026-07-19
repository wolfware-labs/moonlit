//! Permission primitives: allowlist globsets (exec/network), the env-view filter,
//! the filesystem-grant -> preopen-perms mapping, and the WASI context builder.

#![allow(dead_code)] // Public APIs consumed by Phase 6+ tasks

use globset::{Glob, GlobSet, GlobSetBuilder};
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder};

use crate::config::model::FilesystemAccess;
use crate::host::InstanceConfig;

fn build_globset(patterns: &[String]) -> GlobSet {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            b.add(g);
        }
    }
    b.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Allowlist of permitted program names for `moonlit:plugin/process`.
pub fn exec_globset(patterns: &[String]) -> GlobSet {
    build_globset(patterns)
}

/// Allowlist of permitted outgoing-HTTP hosts.
pub fn network_globset(patterns: &[String]) -> GlobSet {
    build_globset(patterns)
}

/// Filter an env snapshot to the keys permitted by the `env` grant.
pub fn filter_env(patterns: &[String], snapshot: &[(String, String)]) -> Vec<(String, String)> {
    let gs = build_globset(patterns);
    snapshot
        .iter()
        .filter(|(k, _)| gs.is_match(k))
        .cloned()
        .collect()
}

/// Map a filesystem grant to `(DirPerms, FilePerms)`; `None` = no preopen at all.
pub fn filesystem_perms(access: FilesystemAccess) -> Option<(DirPerms, FilePerms)> {
    match access {
        FilesystemAccess::None => None,
        FilesystemAccess::ReadOnly => Some((DirPerms::READ, FilePerms::READ)),
        FilesystemAccess::ReadWrite => Some((
            DirPerms::READ | DirPerms::MUTATE,
            FilePerms::READ | FilePerms::WRITE,
        )),
    }
}

/// Build the per-instance WASI context: a filtered env view and a working-dir
/// preopen gated by the `filesystem` grant (`none` => the guest gets no fd at all).
pub fn build_wasi_ctx(cfg: &InstanceConfig) -> anyhow::Result<WasiCtx> {
    let mut b = WasiCtxBuilder::new();
    for (k, v) in filter_env(&cfg.permissions.env, &cfg.env_snapshot) {
        b.env(&k, &v);
    }
    if let Some((dir, file)) = filesystem_perms(cfg.permissions.filesystem) {
        b.preopened_dir(&cfg.working_directory, ".", dir, file)?;
    }
    Ok(b.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::FilesystemAccess;

    #[test]
    fn exec_globset_matches_allowlist() {
        let gs = exec_globset(&["echo".to_string(), "git*".to_string()]);
        assert!(gs.is_match("echo"));
        assert!(gs.is_match("git"));
        assert!(gs.is_match("gitlab"));
        assert!(!gs.is_match("rm"));
    }

    #[test]
    fn star_matches_everything() {
        let gs = network_globset(&["*".to_string()]);
        assert!(gs.is_match("api.github.com"));
        assert!(gs.is_match("example.com"));
    }

    #[test]
    fn filter_env_keeps_only_matching_keys() {
        let snap = vec![
            ("GITHUB_TOKEN".to_string(), "x".to_string()),
            ("PATH".to_string(), "/bin".to_string()),
        ];
        let kept = filter_env(&["GITHUB_*".to_string()], &snap);
        assert_eq!(kept, vec![("GITHUB_TOKEN".to_string(), "x".to_string())]);
    }

    #[test]
    fn filesystem_perms_maps_each_grant() {
        assert!(filesystem_perms(FilesystemAccess::None).is_none());
        let (d, f) = filesystem_perms(FilesystemAccess::ReadOnly).unwrap();
        assert_eq!(d, DirPerms::READ);
        assert_eq!(f, FilePerms::READ);
        let (d, f) = filesystem_perms(FilesystemAccess::ReadWrite).unwrap();
        assert_eq!(d, DirPerms::READ | DirPerms::MUTATE);
        assert_eq!(f, FilePerms::READ | FilePerms::WRITE);
    }
}
