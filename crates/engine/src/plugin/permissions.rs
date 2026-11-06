use crate::pipeline::config::{FilesystemAccess, Permissions};
use crate::plugin::host::{exec_globset, network_globset};
use serde::Serialize;

/// The kind of capability a plugin asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionKind {
  Network,
  Exec,
  Env,
  FilesystemRead,
  FilesystemWrite,
}

impl PermissionKind {
  /// What the permission lets a plugin do, phrased as a verb phrase.
  #[must_use]
  pub fn sentence(self) -> &'static str {
    match self {
      PermissionKind::Network => "Connect to the internet",
      PermissionKind::Exec => "Run programs on your machine",
      PermissionKind::Env => "Read environment variables",
      PermissionKind::FilesystemRead => "Read files in your project",
      PermissionKind::FilesystemWrite => "Read and change files in your project",
    }
  }
}

/// A permission a plugin declares in its metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredPermission {
  pub kind: PermissionKind,
  pub values: Vec<String>,
  pub reason: String,
  pub optional: bool,
}

fn rank(access: FilesystemAccess) -> u8 {
  match access {
    FilesystemAccess::None => 0,
    FilesystemAccess::ReadOnly => 1,
    FilesystemAccess::ReadWrite => 2,
  }
}

fn max_access(a: FilesystemAccess, b: FilesystemAccess) -> FilesystemAccess {
  if rank(b) > rank(a) { b } else { a }
}

fn push_unique(target: &mut Vec<String>, values: &[String]) {
  for v in values {
    if !target.contains(v) {
      target.push(v.clone());
    }
  }
}

impl DeclaredPermission {
  /// The line to add under a plugin's `permissions:` block to grant this declaration.
  #[must_use]
  pub fn yaml_line(&self) -> String {
    let list = || {
      let quoted: Vec<String> = self.values.iter().map(|v| format!("\"{v}\"")).collect();
      format!("[{}]", quoted.join(", "))
    };
    match self.kind {
      PermissionKind::Network => format!("network: {}", list()),
      PermissionKind::Exec => format!("exec: {}", list()),
      PermissionKind::Env => format!("env: {}", list()),
      PermissionKind::FilesystemRead => "filesystem: read-only".to_string(),
      PermissionKind::FilesystemWrite => "filesystem: read-write".to_string(),
    }
  }
}

/// Whether the granted permissions already cover every value of a declared permission.
#[must_use]
pub fn is_covered(p: &DeclaredPermission, granted: &Permissions) -> bool {
  match p.kind {
    PermissionKind::Network => {
      let set = network_globset(&granted.network);
      p.values.iter().all(|v| set.is_match(v))
    }
    PermissionKind::Exec => {
      let set = exec_globset(&granted.exec);
      p.values.iter().all(|v| set.is_match(v))
    }
    PermissionKind::Env => {
      let set = exec_globset(&granted.env);
      p.values.iter().all(|v| set.is_match(v))
    }
    PermissionKind::FilesystemRead => rank(granted.filesystem) >= rank(FilesystemAccess::ReadOnly),
    PermissionKind::FilesystemWrite => rank(granted.filesystem) >= rank(FilesystemAccess::ReadWrite),
  }
}

/// The required (non-optional) declared permissions the granted set does not cover.
#[must_use]
pub fn uncovered_required<'a>(declared: &'a [DeclaredPermission], granted: &Permissions) -> Vec<&'a DeclaredPermission> {
  declared.iter().filter(|p| !p.optional && !is_covered(p, granted)).collect()
}

/// Builds the grants for the required declarations plus the optional ones the predicate accepts.
#[must_use]
pub fn grants_for(declared: &[DeclaredPermission], include_optional: &dyn Fn(&DeclaredPermission) -> bool) -> Permissions {
  let mut out = Permissions::deny();
  for p in declared.iter().filter(|p| !p.optional || include_optional(p)) {
    match p.kind {
      PermissionKind::Network => push_unique(&mut out.network, &p.values),
      PermissionKind::Exec => push_unique(&mut out.exec, &p.values),
      PermissionKind::Env => push_unique(&mut out.env, &p.values),
      PermissionKind::FilesystemRead => out.filesystem = max_access(out.filesystem, FilesystemAccess::ReadOnly),
      PermissionKind::FilesystemWrite => out.filesystem = max_access(out.filesystem, FilesystemAccess::ReadWrite),
    }
  }
  out
}

/// Unions two grant sets, keeping the current order first and the higher filesystem level.
#[must_use]
pub fn merge_grants(current: &Permissions, extra: &Permissions) -> Permissions {
  let mut out = current.clone();
  push_unique(&mut out.network, &extra.network);
  push_unique(&mut out.exec, &extra.exec);
  push_unique(&mut out.env, &extra.env);
  out.filesystem = max_access(out.filesystem, extra.filesystem);
  out
}

/// Whether an environment pattern may expose secrets: it has a wildcard or a secret-looking word.
#[must_use]
pub fn is_secret_like(pattern: &str) -> bool {
  let upper = pattern.to_uppercase();
  pattern.contains('*')
    || ["TOKEN", "KEY", "SECRET", "PASSWORD", "CREDENTIAL"]
      .iter()
      .any(|w| upper.contains(w))
}

/// The first optional declaration of a kind whose values match the target.
#[must_use]
pub fn optional_for_target<'a>(
  declared: &'a [DeclaredPermission],
  kind: PermissionKind,
  target: &str,
) -> Option<&'a DeclaredPermission> {
  declared.iter().filter(|p| p.optional && p.kind == kind).find(|p| {
    let set = exec_globset(&p.values);
    set.is_match(target)
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn decl(kind: PermissionKind, values: &[&str], optional: bool) -> DeclaredPermission {
    DeclaredPermission {
      kind,
      values: values.iter().map(|v| (*v).to_string()).collect(),
      reason: "r".to_string(),
      optional,
    }
  }

  fn granted(network: &[&str], exec: &[&str], env: &[&str], filesystem: FilesystemAccess) -> Permissions {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_string()).collect();
    Permissions {
      network: s(network),
      exec: s(exec),
      env: s(env),
      filesystem,
    }
  }

  #[test]
  fn required_glob_is_covered_by_same_glob_or_star() {
    let p = decl(PermissionKind::Env, &["GITHUB_*"], false);
    assert!(is_covered(&p, &granted(&[], &[], &["GITHUB_*"], FilesystemAccess::None)));
    assert!(is_covered(&p, &granted(&[], &[], &["*"], FilesystemAccess::None)));
    assert!(!is_covered(&p, &granted(&[], &[], &["CI"], FilesystemAccess::None)));
    assert!(!is_covered(&p, &Permissions::deny()));
  }

  #[test]
  fn filesystem_write_not_covered_by_read_only() {
    let w = decl(PermissionKind::FilesystemWrite, &[], false);
    let r = decl(PermissionKind::FilesystemRead, &[], false);
    assert!(!is_covered(&w, &granted(&[], &[], &[], FilesystemAccess::ReadOnly)));
    assert!(is_covered(&w, &granted(&[], &[], &[], FilesystemAccess::ReadWrite)));
    assert!(is_covered(&r, &granted(&[], &[], &[], FilesystemAccess::ReadOnly)));
    assert!(!is_covered(&r, &Permissions::deny()));
  }

  #[test]
  fn optional_entries_are_never_uncovered() {
    let d = vec![
      decl(PermissionKind::Exec, &["git"], true),
      decl(PermissionKind::Network, &["api.example.com"], false),
    ];
    let missing = uncovered_required(&d, &Permissions::deny());
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].kind, PermissionKind::Network);
  }

  #[test]
  fn grants_for_skips_optional_unless_accepted() {
    let d = vec![
      decl(PermissionKind::Exec, &["git"], true),
      decl(PermissionKind::Network, &["a.com", "a.com"], false),
      decl(PermissionKind::FilesystemRead, &[], false),
      decl(PermissionKind::FilesystemWrite, &[], true),
    ];
    let none = grants_for(&d, &|_| false);
    assert!(none.exec.is_empty());
    assert_eq!(none.network, vec!["a.com".to_string()]);
    assert_eq!(none.filesystem, FilesystemAccess::ReadOnly);
    let all = grants_for(&d, &|_| true);
    assert_eq!(all.exec, vec!["git".to_string()]);
    assert_eq!(all.filesystem, FilesystemAccess::ReadWrite);
  }

  #[test]
  fn merge_grants_keeps_current_order_and_max_filesystem() {
    let current = granted(&["b.com"], &["git"], &[], FilesystemAccess::ReadWrite);
    let extra = granted(&["a.com", "b.com"], &["ls"], &["CI"], FilesystemAccess::ReadOnly);
    let merged = merge_grants(&current, &extra);
    assert_eq!(merged.network, vec!["b.com".to_string(), "a.com".to_string()]);
    assert_eq!(merged.exec, vec!["git".to_string(), "ls".to_string()]);
    assert_eq!(merged.env, vec!["CI".to_string()]);
    assert_eq!(merged.filesystem, FilesystemAccess::ReadWrite);
  }

  #[test]
  fn secret_like_patterns() {
    assert!(is_secret_like("GITHUB_*"));
    assert!(is_secret_like("NPM_TOKEN"));
    assert!(is_secret_like("aws_secret_access_key"));
    assert!(!is_secret_like("CI"));
    assert!(!is_secret_like("HOME"));
  }

  #[test]
  fn optional_for_target_matches_glob_values() {
    let d = vec![
      decl(PermissionKind::Exec, &["git"], false),
      decl(PermissionKind::Exec, &["cargo*"], true),
      decl(PermissionKind::Network, &["*.example.com"], true),
    ];
    let hit = optional_for_target(&d, PermissionKind::Exec, "cargo-fmt").unwrap();
    assert_eq!(hit.values, vec!["cargo*".to_string()]);
    assert!(optional_for_target(&d, PermissionKind::Exec, "git").is_none());
    assert!(optional_for_target(&d, PermissionKind::Network, "api.example.com").is_some());
    assert!(optional_for_target(&d, PermissionKind::Env, "X").is_none());
  }
}
