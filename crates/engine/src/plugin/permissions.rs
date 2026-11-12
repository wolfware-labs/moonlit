use crate::pipeline::config::{FilesystemAccess, Permissions};
use crate::plugin::host::{build_globset, exec_globset, network_globset};
use crate::sanitize::quote_list;
use globset::GlobSet;
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

  /// [`PermissionKind::sentence`] with its first letter lowercased, for use inside a longer sentence.
  #[must_use]
  pub fn verb_phrase(self) -> String {
    let mut chars = self.sentence().chars();
    chars
      .next()
      .map(|first| first.to_lowercase().chain(chars).collect())
      .unwrap_or_default()
  }

  /// The key under a plugin's `permissions:` block that grants this kind.
  #[must_use]
  pub fn field(self) -> &'static str {
    match self {
      PermissionKind::Network => "network",
      PermissionKind::Exec => "exec",
      PermissionKind::Env => "env",
      PermissionKind::FilesystemRead | PermissionKind::FilesystemWrite => "filesystem",
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
    let value = match self.kind {
      PermissionKind::Network | PermissionKind::Exec | PermissionKind::Env => quote_list(&self.values),
      PermissionKind::FilesystemRead => FilesystemAccess::ReadOnly.as_str().to_string(),
      PermissionKind::FilesystemWrite => FilesystemAccess::ReadWrite.as_str().to_string(),
    };
    format!("{}: {value}", self.kind.field())
  }
}

/// Why the first malformed declaration breaks the contract `moonlit_plugin!` enforces: a blank reason, no
/// values for `network`, `exec` or `env`, values for `filesystem`, or glob syntax other than `*` and `?`.
#[must_use]
pub fn invalid_declaration(declared: &[DeclaredPermission]) -> Option<String> {
  declared.iter().find_map(|p| {
    let field = p.kind.field();
    let listed = !matches!(p.kind, PermissionKind::FilesystemRead | PermissionKind::FilesystemWrite);
    if p.reason.trim().is_empty() {
      Some(format!("the {field} permission has no reason"))
    } else if listed && p.values.is_empty() {
      Some(format!("the {field} permission lists no values"))
    } else if !listed && !p.values.is_empty() {
      Some(format!("the {field} permission takes no values"))
    } else {
      p.values
        .iter()
        .find(|v| Pattern::parse(v).is_none())
        .map(|v| format!("permission value `{v}` uses `[`, `]`, `{{`, `}}` or `\\`; only `*` and `?` wildcards are allowed"))
    }
  })
}

/// Whether the granted permissions already cover every value of a declared permission.
#[must_use]
pub fn is_covered(p: &DeclaredPermission, granted: &Permissions) -> bool {
  match p.kind {
    PermissionKind::Network => all_granted(&p.values, &granted.network, network_globset),
    PermissionKind::Exec => all_granted(&p.values, &granted.exec, exec_globset),
    PermissionKind::Env => all_granted(&p.values, &granted.env, exec_globset),
    PermissionKind::FilesystemRead => granted.filesystem >= FilesystemAccess::ReadOnly,
    PermissionKind::FilesystemWrite => granted.filesystem >= FilesystemAccess::ReadWrite,
  }
}

fn all_granted(values: &[String], granted: &[String], globset: fn(&[String]) -> GlobSet) -> bool {
  let set = globset(granted);
  values.iter().all(|v| {
    granted.contains(v)
      || match Pattern::parse(v) {
        Some(declared) if declared.is_literal() => set.is_match(v),
        Some(declared) => granted
          .iter()
          .filter_map(|g| Pattern::parse(g))
          .any(|grant| grant.includes(&declared)),
        None => false,
      }
  })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Symbol {
  Char(char),
  One,
  Any,
}

struct Pattern(Vec<Symbol>);

impl Pattern {
  fn parse(glob: &str) -> Option<Self> {
    let mut out = Vec::new();
    let mut ones = 0;
    let mut any = false;
    for c in glob.chars() {
      match c {
        '?' => ones += 1,
        '*' => any = true,
        '[' | ']' | '{' | '}' | '\\' => return None,
        c => {
          flush_wildcards(&mut out, &mut ones, &mut any);
          out.push(Symbol::Char(c));
        }
      }
    }
    flush_wildcards(&mut out, &mut ones, &mut any);
    Some(Self(out))
  }

  fn is_literal(&self) -> bool {
    self.0.iter().all(|s| matches!(s, Symbol::Char(_)))
  }

  fn includes(&self, other: &Self) -> bool {
    let (g, d) = (&self.0, &other.0);
    let mut row = vec![false; d.len() + 1];
    row[d.len()] = true;
    for i in (0..g.len()).rev() {
      let mut next = vec![false; d.len() + 1];
      for j in (0..=d.len()).rev() {
        next[j] = match g[i] {
          Symbol::Any => row[j] || (j < d.len() && next[j + 1]),
          Symbol::One => j < d.len() && d[j] != Symbol::Any && row[j + 1],
          Symbol::Char(c) => j < d.len() && d[j] == Symbol::Char(c) && row[j + 1],
        };
      }
      row = next;
    }
    row[0]
  }
}

fn flush_wildcards(out: &mut Vec<Symbol>, ones: &mut usize, any: &mut bool) {
  out.extend(std::iter::repeat_n(Symbol::One, *ones));
  if *any {
    out.push(Symbol::Any);
  }
  *ones = 0;
  *any = false;
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
      PermissionKind::FilesystemRead => out.filesystem = out.filesystem.max(FilesystemAccess::ReadOnly),
      PermissionKind::FilesystemWrite => out.filesystem = out.filesystem.max(FilesystemAccess::ReadWrite),
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
  out.filesystem = out.filesystem.max(extra.filesystem);
  out
}

/// The granted permissions widened by the required ones they were missing.
#[must_use]
pub fn grant_missing(granted: &Permissions, missing: &[DeclaredPermission]) -> Permissions {
  merge_grants(granted, &grants_for(missing, &|_| false))
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
  declared
    .iter()
    .filter(|p| p.optional && p.kind == kind)
    .find(|p| build_globset(&p.values).is_match(target))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn values_with_unsupported_glob_syntax_are_invalid() {
    for bad in ["{git,sh}", "git[0-9]", "a]", "b}", "foo\\*"] {
      let declared = [
        decl(PermissionKind::Network, &["example.com"], false),
        decl(PermissionKind::Exec, &["git", bad], false),
      ];
      assert_eq!(
        invalid_declaration(&declared),
        Some(format!(
          "permission value `{bad}` uses `[`, `]`, `{{`, `}}` or `\\`; only `*` and `?` wildcards are allowed"
        ))
      );
    }
  }

  #[test]
  fn a_blank_reason_is_invalid() {
    for reason in ["", "  \t"] {
      let mut p = decl(PermissionKind::Exec, &["git"], false);
      p.reason = reason.to_string();
      assert_eq!(
        invalid_declaration(&[p]),
        Some("the exec permission has no reason".to_string()),
        "{reason:?}"
      );
    }
  }

  #[test]
  fn a_listed_kind_without_values_is_invalid() {
    for kind in [PermissionKind::Network, PermissionKind::Exec, PermissionKind::Env] {
      assert_eq!(
        invalid_declaration(&[decl(kind, &[], true)]),
        Some(format!("the {} permission lists no values", kind.field()))
      );
    }
  }

  #[test]
  fn filesystem_with_values_is_invalid() {
    for kind in [PermissionKind::FilesystemRead, PermissionKind::FilesystemWrite] {
      assert_eq!(
        invalid_declaration(&[decl(kind, &["/tmp"], false)]),
        Some("the filesystem permission takes no values".to_string())
      );
    }
  }

  #[test]
  fn well_formed_declarations_are_valid() {
    let declared = [
      decl(PermissionKind::Exec, &["git", "cargo-*"], false),
      decl(PermissionKind::Env, &["GH_?OKEN"], true),
      decl(PermissionKind::Network, &["api.example.com"], false),
      decl(PermissionKind::FilesystemRead, &[], false),
      decl(PermissionKind::FilesystemWrite, &[], true),
    ];
    assert_eq!(invalid_declaration(&declared), None);
  }

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
  fn yaml_line_quotes_values_so_they_parse_back() {
    let values = ["a\"b", "c\\d", "e\u{1b}[2Kf", "line\nbreak"];
    let line = decl(PermissionKind::Exec, &values, false).yaml_line();
    assert!(!line.contains('\u{1b}') && !line.contains('\n'), "{line:?}");
    let parsed: std::collections::HashMap<String, Vec<String>> = serde_yaml_ng::from_str(&line).expect("valid yaml");
    assert_eq!(parsed["exec"], values);
  }

  #[test]
  fn required_glob_is_covered_by_same_glob_or_star() {
    let p = decl(PermissionKind::Env, &["GITHUB_*"], false);
    assert!(is_covered(&p, &granted(&[], &[], &["GITHUB_*"], FilesystemAccess::None)));
    assert!(is_covered(&p, &granted(&[], &[], &["*"], FilesystemAccess::None)));
    assert!(!is_covered(&p, &granted(&[], &[], &["CI"], FilesystemAccess::None)));
    assert!(!is_covered(&p, &Permissions::deny()));
  }

  fn exec_covered(declared: &str, grants: &[&str]) -> bool {
    is_covered(
      &decl(PermissionKind::Exec, &[declared], false),
      &granted(&[], grants, &[], FilesystemAccess::None),
    )
  }

  #[test]
  fn a_grant_covers_a_declared_glob_only_when_it_matches_everything_the_glob_does() {
    assert!(!exec_covered("a*c", &["a?c"]));
    assert!(!exec_covered("a?c", &["abc"]));
    assert!(!exec_covered("a*", &["a?*"]));
    assert!(!exec_covered("*", &["a*"]));
    assert!(!exec_covered("a*c", &["a*d"]));
    assert!(!exec_covered("a?", &["a?c"]));
    assert!(!exec_covered("ab*", &["ab"]));
    assert!(exec_covered("a*c", &["a*"]));
    assert!(exec_covered("a*c", &["*c"]));
    assert!(exec_covered("a*c", &["*"]));
    assert!(exec_covered("a?c", &["a*c"]));
    assert!(exec_covered("abc?", &["a?c?"]));
    assert!(exec_covered("a**c", &["a*c"]));
    assert!(exec_covered("x*?", &["x?*"]));
    assert!(exec_covered("x?*", &["x*?"]));
    assert!(exec_covered("a*c", &["b*", "a*c*"]));
  }

  #[test]
  fn a_declared_glob_is_not_covered_by_a_grant_with_classes_or_alternatives() {
    assert!(!exec_covered("g*", &["{g*,h*}"]));
    assert!(!exec_covered("g*", &["[g]*"]));
    assert!(!exec_covered("g*", &["\\g*"]));
    assert!(exec_covered("gh", &["{git,gh}"]));
    assert!(exec_covered("[g]*", &["[g]*"]));
    assert!(!exec_covered("[g]*", &["*"]));
  }

  #[test]
  fn a_declared_value_equal_to_a_granted_value_is_covered() {
    let braces = decl(PermissionKind::Exec, &["{git,gh}"], false);
    assert!(is_covered(&braces, &granted(&[], &["{git,gh}"], &[], FilesystemAccess::None)));
    let invalid = decl(PermissionKind::Network, &["[api.example.com"], false);
    assert!(is_covered(
      &invalid,
      &granted(&["[api.example.com"], &[], &[], FilesystemAccess::None)
    ));
    let mixed = decl(PermissionKind::Env, &["CI", "[X"], false);
    assert!(is_covered(&mixed, &granted(&[], &[], &["[X", "C*"], FilesystemAccess::None)));
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
  fn every_uncovered_required_entry_of_one_plugin_is_listed_in_declaration_order() {
    let d = vec![
      decl(PermissionKind::Network, &["api.example.com"], false),
      decl(PermissionKind::Exec, &["git"], false),
      decl(PermissionKind::Env, &["HOME"], true),
    ];
    let kinds: Vec<PermissionKind> = uncovered_required(&d, &Permissions::deny()).iter().map(|p| p.kind).collect();
    assert_eq!(kinds, vec![PermissionKind::Network, PermissionKind::Exec]);
    let mut granted = Permissions::deny();
    granted.exec = vec!["git".to_string()];
    let kinds: Vec<PermissionKind> = uncovered_required(&d, &granted).iter().map(|p| p.kind).collect();
    assert_eq!(kinds, vec![PermissionKind::Network]);
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

  #[test]
  fn optional_for_target_matches_network_hosts_with_network_semantics() {
    let d = vec![
      decl(PermissionKind::Exec, &["api.example.com"], true),
      decl(PermissionKind::Network, &["*.example.com", "cdn.test"], true),
    ];
    let hit = optional_for_target(&d, PermissionKind::Network, "api.example.com").unwrap();
    assert_eq!(hit.kind, PermissionKind::Network);
    assert!(optional_for_target(&d, PermissionKind::Network, "cdn.test").is_some());
    assert!(optional_for_target(&d, PermissionKind::Network, "other.org").is_none());
  }

  #[test]
  fn verb_phrase_lowercases_the_first_letter_only() {
    assert_eq!(PermissionKind::Network.verb_phrase(), "connect to the internet");
    assert_eq!(
      PermissionKind::FilesystemWrite.verb_phrase(),
      "read and change files in your project"
    );
  }

  #[test]
  fn read_grant_line_and_env_grants() {
    let line = decl(PermissionKind::FilesystemRead, &[], false).yaml_line();
    assert!(line.ends_with(": read-only"), "{line}");
    let grants = grants_for(&[decl(PermissionKind::Env, &["A", "A", "B"], false)], &|_| false);
    assert_eq!(grants.env, ["A", "B"]);
  }
}
