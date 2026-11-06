use std::fmt::Write;
use std::io::IsTerminal;

use console::Style;
use moonlit_engine::plugin::permissions::{DeclaredPermission, PermissionKind, is_secret_like};
use moonlit_engine::registry::VerificationStatus;

use crate::commands::search::badge;

const KINDS: [PermissionKind; 5] = [
  PermissionKind::Network,
  PermissionKind::Exec,
  PermissionKind::Env,
  PermissionKind::FilesystemRead,
  PermissionKind::FilesystemWrite,
];

/// Everything the consent screen shows about one plugin version.
pub struct ConsentView<'a> {
  pub plugin: &'a str,
  pub version: &'a str,
  pub status: Option<VerificationStatus>,
  pub description: &'a str,
  pub required: Vec<&'a DeclaredPermission>,
  pub optional: Vec<&'a DeclaredPermission>,
}

/// The user's answer to a consent question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
  Yes,
  No,
}

/// Renders the consent screen as text, with ANSI styling when `color` is set.
#[must_use]
pub fn render(view: &ConsentView, color: bool) -> String {
  let paint = |text: &str, style: Style| {
    if color {
      style.force_styling(true).apply_to(text).to_string()
    } else {
      text.to_string()
    }
  };
  let mut out = String::new();

  let mut identity = format!("  {} {}", paint(view.plugin, Style::new().bold()), view.version);
  if let Some(status) = view.status {
    let label = badge(status);
    let painted = if status == VerificationStatus::Verified {
      paint(label, Style::new().green())
    } else {
      paint(label, Style::new().dim())
    };
    let _ = write!(identity, "   {painted}");
  }
  let _ = writeln!(out, "{identity}");
  if !view.description.is_empty() {
    let _ = writeln!(out, "  {}", view.description);
  }
  out.push('\n');

  if view.required.is_empty() && view.optional.is_empty() {
    let _ = writeln!(out, "  This plugin needs no permissions.");
    out.push('\n');
  } else {
    if !view.required.is_empty() {
      let _ = writeln!(out, "  {}", paint("This plugin wants to:", Style::new().bold()));
      out.push('\n');
      for p in &view.required {
        write_entry(&mut out, p, &paint);
      }
    }
    if !view.optional.is_empty() {
      let _ = writeln!(out, "  {}", paint("Optional:", Style::new().bold()));
      out.push('\n');
      for p in &view.optional {
        write_entry(&mut out, p, &paint);
      }
    }
    let missing: Vec<String> = KINDS
      .iter()
      .filter(|k| !view.required.iter().chain(view.optional.iter()).any(|p| p.kind == **k))
      .map(|k| lower_first(k.sentence()))
      .collect();
    if !missing.is_empty() {
      let _ = writeln!(out, "  Not requested: {}", missing.join(", "));
      out.push('\n');
    }
  }

  out
}

/// Renders the changes a consent would make to `file`, as listed by `release_file::render_diff`.
#[must_use]
pub fn render_changes(file: &str, diff: &str) -> String {
  format!("  {file}\n{diff}\n")
}

fn lower_first(sentence: &str) -> String {
  let mut chars = sentence.chars();
  match chars.next() {
    Some(first) => first.to_lowercase().chain(chars).collect(),
    None => String::new(),
  }
}

fn icon(kind: PermissionKind) -> &'static str {
  match kind {
    PermissionKind::Network => "🌐",
    PermissionKind::Exec => "🔧",
    PermissionKind::Env => "🔑",
    PermissionKind::FilesystemRead | PermissionKind::FilesystemWrite => "📁",
  }
}

/// The values of a permission as shown to the user.
#[must_use]
pub fn values_text(p: &DeclaredPermission) -> String {
  match p.kind {
    PermissionKind::FilesystemRead => "read-only".to_string(),
    PermissionKind::FilesystemWrite => "read-write".to_string(),
    _ => p.values.join(", "),
  }
}

fn write_entry(out: &mut String, p: &DeclaredPermission, paint: &dyn Fn(&str, Style) -> String) {
  let width = KINDS.iter().map(|k| k.sentence().chars().count()).max().unwrap_or(0) + 2;
  let _ = writeln!(out, "  {} {:<width$}{}", icon(p.kind), p.kind.sentence(), values_text(p));
  let _ = writeln!(out, "     {}", paint(&format!("\"{}\"", p.reason), Style::new().dim()));
  if p.kind == PermissionKind::Env && p.values.iter().any(|v| is_secret_like(v)) {
    let _ = writeln!(out, "     {}", paint("⚠ may include secrets", Style::new().yellow()));
  }
  out.push('\n');
}

/// Asks a yes/no question, defaulting to no.
pub fn ask(question: &str) -> std::io::Result<Answer> {
  let yes = cliclack::confirm(question).initial_value(false).interact()?;
  Ok(if yes { Answer::Yes } else { Answer::No })
}

/// Asks whether to also grant one optional permission.
pub fn ask_optional(p: &DeclaredPermission) -> std::io::Result<Answer> {
  ask(&format!("Also allow: {} ({})?", p.kind.sentence(), values_text(p)))
}

/// Whether stdin is a terminal, so prompts can be answered.
#[must_use]
pub fn interactive() -> bool {
  std::io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn perm(kind: PermissionKind, values: &[&str], reason: &str, optional: bool) -> DeclaredPermission {
    DeclaredPermission {
      kind,
      values: values.iter().map(|v| (*v).to_string()).collect(),
      reason: reason.to_string(),
      optional,
    }
  }

  fn github() -> Vec<DeclaredPermission> {
    vec![
      perm(
        PermissionKind::Network,
        &["api.github.com", "uploads.github.com"],
        "Creates the release and uploads its assets.",
        false,
      ),
      perm(
        PermissionKind::Env,
        &["GITHUB_*"],
        "Reads GITHUB_TOKEN to call the API.",
        false,
      ),
      perm(
        PermissionKind::FilesystemRead,
        &[],
        "Reads the changelog and built artifacts.",
        false,
      ),
    ]
  }

  fn view<'a>(required: Vec<&'a DeclaredPermission>, optional: Vec<&'a DeclaredPermission>) -> ConsentView<'a> {
    ConsentView {
      plugin: "wolfware/github",
      version: "2.0.0",
      status: Some(VerificationStatus::Verified),
      description: "Create GitHub releases and comment on PRs",
      required,
      optional,
    }
  }

  #[test]
  fn renders_required_with_reasons_and_secret_flag() {
    let perms = github();
    let out = render(&view(perms.iter().collect(), vec![]), false);
    assert!(out.contains("wolfware/github 2.0.0"));
    assert!(out.contains("✓ verified"));
    assert!(out.contains("This plugin wants to:"));
    assert!(out.contains("Connect to the internet"));
    assert!(out.contains("api.github.com, uploads.github.com"));
    assert!(out.contains("\"Creates the release and uploads its assets.\""));
    assert!(out.contains("read-only"));
    assert_eq!(out.matches("⚠ may include secrets").count(), 1);
    let lines: Vec<&str> = out.lines().collect();
    let env = lines.iter().position(|l| l.contains("Read environment variables")).unwrap();
    assert!(lines[env + 1].contains("Reads GITHUB_TOKEN"));
    assert!(lines[env + 2].contains("⚠ may include secrets"));
  }

  #[test]
  fn screen_renders_without_a_diff_section() {
    let perms = github();
    let out = render(&view(perms.iter().collect(), vec![]), false);
    assert!(!out.contains("release.yml"), "{out}");
    assert!(!out.contains("+ "), "{out}");
  }

  #[test]
  fn changes_list_the_diff_under_the_file_at_the_same_indent() {
    let out = render_changes("release.yml", "  + plugins:\n  +   - name: github\n");
    assert_eq!(out, "  release.yml\n  + plugins:\n  +   - name: github\n\n");
  }

  #[test]
  fn renders_optional_group() {
    let perms = github();
    let opt = perm(PermissionKind::FilesystemWrite, &[], "Only for write-variables.", true);
    let out = render(&view(perms.iter().collect(), vec![&opt]), false);
    let at = out.find("Optional:").expect("optional heading");
    assert!(out[at..].contains("Read and change files in your project"));
    assert!(out[at..].contains("read-write"));
    assert!(out[at..].contains("\"Only for write-variables.\""));
    assert!(!out[..at].contains("Read and change files"));
  }

  #[test]
  fn not_requested_lists_missing_kinds() {
    let perms = github();
    let out = render(&view(perms.iter().collect(), vec![]), false);
    assert!(
      out.contains("Not requested: run programs on your machine, read and change files in your project"),
      "{out}"
    );
  }

  #[test]
  fn sentence_and_value_keep_a_gap() {
    for (kind, value) in [
      (PermissionKind::FilesystemWrite, "read-write"),
      (PermissionKind::Network, "api.github.com"),
    ] {
      let p = perm(kind, &["api.github.com"], "Why.", false);
      let out = render(&view(vec![&p], vec![]), false);
      let line = out.lines().find(|l| l.contains(kind.sentence())).unwrap();
      let after = line.split(kind.sentence()).nth(1).unwrap();
      assert!(after.starts_with("  "), "{line}");
      assert!(after.trim_start().starts_with(value), "{line}");
    }
  }

  #[test]
  fn no_permissions_says_so() {
    let out = render(&view(vec![], vec![]), false);
    assert!(out.contains("This plugin needs no permissions."));
    assert!(!out.contains("wants to"));
  }
}
