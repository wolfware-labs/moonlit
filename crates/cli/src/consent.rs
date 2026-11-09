use std::fmt::Write;
use std::io::IsTerminal;

use console::Style;
use moonlit_engine::plugin::permissions::{DeclaredPermission, PermissionKind, is_secret_like};
use moonlit_engine::registry::VerificationStatus;

use crate::display::{badge, display_safe};
use crate::release_file::render_diff;

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
  /// Whether to list the kinds the plugin does not ask for; off on screens that show only new permissions.
  pub not_requested: bool,
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

  let mut identity = format!(
    "  {} {}",
    paint(&display_safe(view.plugin), Style::new().bold()),
    display_safe(view.version)
  );
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
    let _ = writeln!(out, "  {}", display_safe(view.description));
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
      .map(|k| k.verb_phrase())
      .collect();
    if view.not_requested && !missing.is_empty() {
      let _ = writeln!(out, "  Not requested: {}", missing.join(", "));
      out.push('\n');
    }
  }

  out
}

/// Renders the lines a consent would change in `file`, as listed by [`render_diff`].
#[must_use]
pub fn render_changes(file: &str, before: &str, after: &str) -> String {
  format!("  {file}\n{}\n", render_diff(before, after))
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
    _ => display_safe(&p.values.join(", ")),
  }
}

fn write_entry(out: &mut String, p: &DeclaredPermission, paint: &dyn Fn(&str, Style) -> String) {
  let width = KINDS.iter().map(|k| k.sentence().chars().count()).max().unwrap_or(0) + 2;
  let _ = writeln!(out, "  {} {:<width$}{}", icon(p.kind), p.kind.sentence(), values_text(p));
  let _ = writeln!(
    out,
    "     {}",
    paint(&format!("\"{}\"", display_safe(&p.reason)), Style::new().dim())
  );
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
      not_requested: true,
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
    assert!(out.contains("  🌐 Connect to the internet"), "{out}");
    assert!(out.contains("  🔑 Read environment variables"), "{out}");
    assert!(out.contains("  📁 Read files in your project"), "{out}");
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
    let out = render_changes("release.yml", "name: a\n", "name: a\nplugins:\n  - name: github\n");
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
  fn delta_screen_has_no_not_requested_line() {
    let perms = github();
    let mut delta = view(vec![&perms[0]], vec![]);
    delta.not_requested = false;
    let out = render(&delta, false);
    assert!(out.contains("Connect to the internet"), "{out}");
    assert!(!out.contains("Not requested"), "{out}");
  }

  #[test]
  fn control_characters_from_the_plugin_render_escaped() {
    let p = perm(PermissionKind::Exec, &["git\u{1b}[8m"], "Tags.\u{1b}[1A\u{1b}[2K", false);
    let mut screen = view(vec![&p], vec![]);
    screen.plugin = "evil\u{9b}31m";
    screen.version = "1.0.0\u{7}";
    screen.description = "Hides\u{1b}[8m";
    let out = render(&screen, false);
    assert!(!out.chars().any(|c| c.is_control() && c != '\n'), "{out:?}");
    assert!(out.contains("\"Tags.\\x1b[1A\\x1b[2K\""), "{out}");
    assert!(out.contains("git\\x1b[8m"), "{out}");
    assert!(out.contains("evil\\x9b31m 1.0.0\\x07"), "{out}");
    assert!(out.contains("Hides\\x1b[8m"), "{out}");
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
  fn values_line_up_two_spaces_after_the_longest_sentence() {
    let p = perm(PermissionKind::FilesystemWrite, &[], "Why.", false);
    let out = render(&view(vec![&p], vec![]), false);
    assert!(
      out.contains("  📁 Read and change files in your project  read-write\n"),
      "{out}"
    );
  }

  #[test]
  fn only_secret_like_env_values_carry_the_secret_warning() {
    let env = perm(PermissionKind::Env, &["HOME"], "Why.", false);
    let network = perm(PermissionKind::Network, &["TOKEN.example.com"], "Why.", false);
    let out = render(&view(vec![&env, &network], vec![]), false);
    assert!(!out.contains("may include secrets"), "{out}");
  }

  #[test]
  fn colored_badge_is_green_only_when_verified() {
    let verified = render(&view(vec![], vec![]), true);
    assert!(verified.contains("\u{1b}[32m✓ verified"), "{verified:?}");
    let mut pending = view(vec![], vec![]);
    pending.status = Some(VerificationStatus::Pending);
    let pending = render(&pending, true);
    assert!(pending.contains("\u{1b}[2mpending"), "{pending:?}");
  }

  #[test]
  fn no_permissions_says_so() {
    let out = render(&view(vec![], vec![]), false);
    assert!(out.contains("This plugin needs no permissions."));
    assert!(!out.contains("wants to"));
  }

  #[test]
  fn only_optional_permissions_skip_the_required_heading() {
    let optional = perm(PermissionKind::Network, &["api.example.com"], "Why.", true);
    let mut unverified = view(vec![], vec![&optional]);
    unverified.status = Some(VerificationStatus::Unverified);
    let out = render(&unverified, false);
    assert!(out.contains("Optional:"), "{out}");
    assert!(!out.contains("wants to"), "{out}");
    assert!(out.contains("unverified"), "{out}");
  }
}
