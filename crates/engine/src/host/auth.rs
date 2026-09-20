use globset::{Glob, GlobSet, GlobSetBuilder};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder};

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

pub fn exec_globset(patterns: &[String]) -> GlobSet {
    build_globset(patterns)
}

pub fn network_globset(patterns: &[String]) -> GlobSet {
    build_globset(patterns)
}

pub fn filter_env(patterns: &[String], snapshot: &[(String, String)]) -> Vec<(String, String)> {
    let gs = build_globset(patterns);
    snapshot
        .iter()
        .filter(|(k, _)| gs.is_match(k))
        .cloned()
        .collect()
}

pub fn filesystem_perms(access: FilesystemAccess) -> Option<FsPerms> {
    match access {
        FilesystemAccess::None => None,
        FilesystemAccess::ReadOnly => Some(FsPerms::ReadOnly),
        FilesystemAccess::ReadWrite => Some(FsPerms::ReadWrite),
    }
}

pub fn build_wasi_ctx(cfg: &InstanceConfig) -> anyhow::Result<WasiCtx> {
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
        assert_eq!(
            filesystem_perms(FilesystemAccess::ReadOnly),
            Some(FsPerms::ReadOnly)
        );
        assert_eq!(
            filesystem_perms(FilesystemAccess::ReadWrite),
            Some(FsPerms::ReadWrite)
        );
    }
}
