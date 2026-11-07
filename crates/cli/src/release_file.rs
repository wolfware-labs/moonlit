use moonlit_engine::pipeline::config::{
  FilesystemAccess, Permissions, PipelineConfig, PluginDeclaration, parse_config, parse_config_unvalidated,
};
use moonlit_engine::plugin::permissions::PermissionKind;
use moonlit_engine::sanitize::{quote, quote_list};
use serde_yaml_ng::{Mapping, Value};

/// A plugin entry to write into a release file.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginEntry {
  pub name: String,
  pub url: String,
  pub permissions: Permissions,
}

/// Why a release file could not be edited.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ReleaseFileError {
  /// Another plugin entry already uses this name; holds that entry's URL.
  #[error("a plugin named '{name}' is already declared")]
  #[diagnostic(code(moonlit::release_file::name_taken))]
  NameTaken { name: String, url: String },
  /// A plugin entry with this URL already exists under the given name.
  #[error("this plugin is already declared as '{name}'")]
  #[diagnostic(code(moonlit::release_file::url_present))]
  UrlPresent { name: String },
  /// No plugin entry has this name.
  #[error("no plugin named '{0}' is declared")]
  #[diagnostic(code(moonlit::release_file::plugin_not_found))]
  PluginNotFound(String),
  /// The file cannot be read, or the edited file would not parse back to the requested entry.
  #[error("the edited release file would be invalid: {0}")]
  #[diagnostic(code(moonlit::release_file::invalid))]
  Invalid(String),
}

/// Appends a plugin entry to the `plugins:` sequence, leaving every other byte of the file unchanged.
pub fn add_plugin(content: &str, entry: &PluginEntry) -> Result<String, ReleaseFileError> {
  check_addable(content, &entry.name, &entry.url)?;
  let config = parse_structure(content)?;
  let plugins = &config.plugins.value;
  let edit = if let Some(last) = plugins.last() {
    let dash = column(content, last.span.start);
    let key = key_column(content, last.span.start);
    insertion(content, line_end(content, last.span.end), item_block(entry, dash, key))
  } else if top_level_line(content, "plugins:").is_some() {
    return Err(ReleaseFileError::Invalid(
      "the plugins key holds no block sequence to add to".into(),
    ));
  } else {
    let block = format!("plugins:\n{}", item_block(entry, 2, 4));
    match top_level_line(content, "stages:") {
      Some(at) => insertion(content, at, format!("{block}\n")),
      None => insertion(content, content.len(), block),
    }
  };
  validated(
    content,
    apply(content, vec![edit]),
    &entry.name,
    &entry.url,
    Some(&entry.permissions),
  )
}

/// Checks that a plugin entry with this name and URL could be added, without editing anything.
pub fn check_addable(content: &str, name: &str, url: &str) -> Result<(), ReleaseFileError> {
  let config = parse_structure(content)?;
  let plugins = &config.plugins.value;
  if let Some(p) = plugins.iter().find(|p| p.name == name) {
    return Err(ReleaseFileError::NameTaken {
      name: p.name.clone(),
      url: p.url.value.as_str().to_string(),
    });
  }
  if let Some(p) = plugins.iter().find(|p| p.url.value.as_str() == url) {
    return Err(ReleaseFileError::UrlPresent { name: p.name.clone() });
  }
  Ok(())
}

/// Replaces the `url` of one plugin entry, and its `permissions` when given, leaving every other byte of the
/// file unchanged.
pub fn set_plugin(content: &str, name: &str, url: &str, permissions: Option<&Permissions>) -> Result<String, ReleaseFileError> {
  let config = parse_checked(content)?;
  let plugin = config
    .plugins
    .value
    .iter()
    .find(|p| p.name == name)
    .ok_or_else(|| ReleaseFileError::PluginNotFound(name.to_string()))?;
  let mut edits = Vec::new();
  if plugin.url.value.as_str() != url {
    edits.push((plugin.url.span.start, plugin.url.span.end, scalar(url)));
  }
  if let Some(permissions) = permissions {
    edits.push(permissions_edit(content, plugin, permissions));
  }
  validated(
    content,
    apply(content, edits),
    name,
    url,
    permissions.or(plugin.permissions.as_ref()),
  )
}

/// Lists the lines that differ between two versions of a file, prefixed with `+ ` or `- `.
#[must_use]
pub fn render_diff(before: &str, after: &str) -> String {
  let a: Vec<&str> = before.lines().collect();
  let b: Vec<&str> = after.lines().collect();
  let mut common = vec![vec![0usize; b.len() + 1]; a.len() + 1];
  for i in (0..a.len()).rev() {
    for j in (0..b.len()).rev() {
      common[i][j] = if a[i] == b[j] {
        common[i + 1][j + 1] + 1
      } else {
        common[i + 1][j].max(common[i][j + 1])
      };
    }
  }
  let mut out = Vec::new();
  let (mut i, mut j) = (0, 0);
  while i < a.len() || j < b.len() {
    if i < a.len() && j < b.len() && a[i] == b[j] {
      i += 1;
      j += 1;
    } else if j == b.len() || (i < a.len() && common[i + 1][j] >= common[i][j + 1]) {
      out.push(format!("  - {}\n", a[i]));
      i += 1;
    } else {
      out.push(format!("  + {}\n", b[j]));
      j += 1;
    }
  }
  out.concat()
}

const SOURCE: &str = "release.yml";

type Edit = (usize, usize, String);

fn parse_checked(content: &str) -> Result<PipelineConfig, ReleaseFileError> {
  parse_config(content, SOURCE).map_err(|err| ReleaseFileError::Invalid(err.message().into()))
}

fn parse_structure(content: &str) -> Result<PipelineConfig, ReleaseFileError> {
  parse_config_unvalidated(content, SOURCE).map_err(|err| ReleaseFileError::Invalid(err.message().into()))
}

fn permissions_edit(content: &str, plugin: &PluginDeclaration, permissions: &Permissions) -> Edit {
  if let Some(span) = plugin.permissions_span {
    (
      span.start,
      span.end,
      permissions_block(permissions, column(content, span.start)),
    )
  } else {
    let key = key_column(content, plugin.span.start);
    let lines = format!("{}{}\n", " ".repeat(key), permissions_block(permissions, key));
    insertion(content, line_end(content, plugin.span.end), lines)
  }
}

fn validated(
  before: &str,
  content: String,
  name: &str,
  url: &str,
  permissions: Option<&Permissions>,
) -> Result<String, ReleaseFileError> {
  let config = parse_structure(&content)?;
  if !config.stages.value.is_empty() {
    parse_checked(&content)?;
  }
  let Some(plugin) = config
    .plugins
    .value
    .iter()
    .find(|p| p.name == name && p.url.value.as_str() == url)
  else {
    return Err(ReleaseFileError::Invalid(format!(
      "plugin '{name}' would not read back with the URL '{url}'"
    )));
  };
  if plugin.permissions.as_ref() != permissions {
    return Err(ReleaseFileError::Invalid(format!(
      "plugin '{name}' would not read back with the requested permissions"
    )));
  }
  if let Ok((rest_before, entry_before)) = outside_entry(before, name) {
    let (rest_after, entry_after) = outside_entry(&content, name)?;
    if rest_after != rest_before || entry_before.is_some_and(|entry| Some(entry) != entry_after) {
      return Err(ReleaseFileError::Invalid(format!(
        "the edit would change more than the url and permissions of plugin '{name}'"
      )));
    }
  }
  Ok(content)
}

fn outside_entry(content: &str, name: &str) -> Result<(Value, Option<Value>), ReleaseFileError> {
  let mut document: Value = serde_yaml_ng::from_str(content).map_err(|e| ReleaseFileError::Invalid(e.to_string()))?;
  if document.is_null() {
    document = Value::Mapping(Mapping::new());
  }
  let Some(root) = document.as_mapping_mut() else {
    return Ok((document, None));
  };
  let position = parse_structure(content)?.plugins.value.iter().position(|p| p.name == name);
  let mut entry = None;
  if let Some(Value::Sequence(items)) = root.get_mut("plugins") {
    if let Some(index) = position.filter(|&index| index < items.len()) {
      let mut removed = items.remove(index);
      if let Some(fields) = removed.as_mapping_mut() {
        fields.remove("url");
        fields.remove("permissions");
      }
      entry = Some(removed);
    }
    if items.is_empty() {
      root.remove("plugins");
    }
  }
  Ok((document, entry))
}

#[must_use]
fn apply(content: &str, mut edits: Vec<Edit>) -> String {
  edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
  let crlf = content.contains("\r\n");
  let mut out = content.to_string();
  for (start, end, text) in edits {
    let text = if crlf { text.replace('\n', "\r\n") } else { text };
    out.replace_range(start..end, &text);
  }
  out
}

#[must_use]
fn insertion(content: &str, at: usize, lines: String) -> Edit {
  let text = if at == content.len() && !content.is_empty() && !content.ends_with('\n') {
    format!("\n{lines}")
  } else {
    lines
  };
  (at, at, text)
}

#[must_use]
fn line_end(content: &str, pos: usize) -> usize {
  content[pos..].find('\n').map_or(content.len(), |i| pos + i + 1)
}

#[must_use]
fn column(content: &str, pos: usize) -> usize {
  pos - content[..pos].rfind('\n').map_or(0, |i| i + 1)
}

#[must_use]
fn key_column(content: &str, item_start: usize) -> usize {
  let col = column(content, item_start);
  let Some(rest) = content[item_start..].strip_prefix('-') else {
    return col;
  };
  let mut chars = rest.char_indices();
  while let Some((i, c)) = chars.next() {
    match c {
      ' ' | '\t' | '\r' | '\n' => {}
      '#' => {
        chars.by_ref().find(|(_, c)| *c == '\n');
      }
      _ => return column(content, item_start + 1 + i),
    }
  }
  col + 2
}

#[must_use]
fn top_level_line(content: &str, prefix: &str) -> Option<usize> {
  let mut offset = 0;
  for line in content.split_inclusive('\n') {
    if line.starts_with(prefix) {
      return Some(offset);
    }
    offset += line.len();
  }
  None
}

#[must_use]
fn item_block(entry: &PluginEntry, dash: usize, key: usize) -> String {
  let gap = key.saturating_sub(dash + 1).max(1);
  let key = dash + 1 + gap;
  let pad = " ".repeat(key);
  format!(
    "{}-{}name: {}\n{pad}url: {}\n{pad}{}\n",
    " ".repeat(dash),
    " ".repeat(gap),
    scalar(&entry.name),
    scalar(&entry.url),
    permissions_block(&entry.permissions, key)
  )
}

#[must_use]
fn permissions_block(permissions: &Permissions, key: usize) -> String {
  let mut lines = Vec::new();
  for (kind, values) in [
    (PermissionKind::Network, &permissions.network),
    (PermissionKind::Exec, &permissions.exec),
    (PermissionKind::Env, &permissions.env),
  ] {
    if !values.is_empty() {
      lines.push(format!("{}: {}", kind.field(), quote_list(values)));
    }
  }
  if permissions.filesystem != FilesystemAccess::None {
    lines.push(format!(
      "{}: {}",
      PermissionKind::FilesystemRead.field(),
      permissions.filesystem.as_str()
    ));
  }
  if lines.is_empty() {
    return "permissions: {}".into();
  }
  let pad = " ".repeat(key + 2);
  let body: Vec<String> = lines.iter().map(|line| format!("{pad}{line}")).collect();
  format!("permissions:\n{}", body.join("\n"))
}

#[must_use]
fn scalar(value: &str) -> String {
  let plain = value.starts_with(|c: char| c.is_ascii_alphanumeric())
    && !value.ends_with(':')
    && !matches!(value, "null" | "Null" | "NULL")
    && value.chars().all(|c| c.is_ascii_alphanumeric() || "._/:@+-=%?&~".contains(c));
  if plain { value.to_string() } else { quote(value) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn control_characters_are_escaped_in_the_written_file() {
    let mut entry = github();
    entry.permissions.network = vec!["api.github.com\u{1b}[8m".into()];
    let out = add_plugin(RELEASE, &entry).expect("added");
    assert!(!out.contains('\u{1b}'), "{out:?}");
    assert!(out.contains("network: [\"api.github.com\\x1b[8m\"]"), "{out}");
  }

  #[test]
  fn read_back_with_other_permissions_is_invalid() {
    let url = "oci://registry.moonlit.rs/moonlit/git:1.2.0";
    let broader = Permissions {
      exec: vec!["git".into(), "sh".into()],
      ..Permissions::deny()
    };
    let err = validated(RELEASE, RELEASE.to_string(), "git", url, Some(&broader)).expect_err("permissions differ");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");
    let err = validated(RELEASE, RELEASE.to_string(), "git", url, None).expect_err("permissions differ");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");
    let granted = Permissions {
      exec: vec!["git".into()],
      ..Permissions::deny()
    };
    assert!(validated(RELEASE, RELEASE.to_string(), "git", url, Some(&granted)).is_ok());
  }

  #[test]
  fn set_plugin_without_permissions_changes_only_the_url() {
    let out = set_plugin(RELEASE, "git", "oci://registry.moonlit.rs/moonlit/git:1.3.0", None).unwrap();
    assert_eq!(out, RELEASE.replace("git:1.2.0", "git:1.3.0"));
    assert!(matches!(
      set_plugin(RELEASE, "nope", "oci://h/o/n:1.0.0", None),
      Err(ReleaseFileError::PluginNotFound(_))
    ));
  }

  const RELEASE: &str = "# Release pipeline for acme
name: acme

plugins:
  # the git plugin
  - name: git
    url: oci://registry.moonlit.rs/moonlit/git:1.2.0
    permissions:
      exec: [\"git\"]

  # - name: old
  #   url: file://old.wasm

stages:
  release:
    - run: git.tag
";

  const GITHUB_ITEM: &str = "  - name: github
    url: oci://registry.moonlit.rs/moonlit/github:2.0.0
    permissions:
      network: [\"api.github.com\"]
      env: [\"GITHUB_TOKEN\"]
      filesystem: read-only
";

  fn github() -> PluginEntry {
    PluginEntry {
      name: "github".into(),
      url: "oci://registry.moonlit.rs/moonlit/github:2.0.0".into(),
      permissions: Permissions {
        network: vec!["api.github.com".into()],
        exec: vec![],
        env: vec!["GITHUB_TOKEN".into()],
        filesystem: FilesystemAccess::ReadOnly,
      },
    }
  }

  fn added_release() -> String {
    let anchor = "      exec: [\"git\"]\n";
    RELEASE.replacen(anchor, &format!("{anchor}{GITHUB_ITEM}"), 1)
  }

  #[test]
  fn add_preserves_comments_and_other_entries() {
    let out = add_plugin(RELEASE, &github()).expect("added");
    assert_eq!(out, added_release());
  }

  #[test]
  fn add_rejects_taken_name() {
    let mut entry = github();
    entry.name = "git".into();
    let err = add_plugin(RELEASE, &entry).expect_err("name taken");
    assert!(
      matches!(err, ReleaseFileError::NameTaken { ref name, .. } if name == "git"),
      "{err:?}"
    );
  }

  #[test]
  fn add_rejects_url_already_present() {
    let mut entry = github();
    entry.url = "oci://registry.moonlit.rs/moonlit/git:1.2.0".into();
    let err = add_plugin(RELEASE, &entry).expect_err("url present");
    assert!(
      matches!(err, ReleaseFileError::UrlPresent { ref name } if name == "git"),
      "{err:?}"
    );
  }

  #[test]
  fn add_creates_plugins_block_when_missing() {
    let input = "# no plugins yet\nname: acme\n\nstages:\n  release:\n    - run: a.go\n";
    let entry = PluginEntry {
      name: "a".into(),
      url: "file://a.wasm".into(),
      permissions: Permissions::deny(),
    };
    let out = add_plugin(input, &entry).expect("added");
    assert_eq!(
      out,
      "# no plugins yet\nname: acme\n\nplugins:\n  - name: a\n    url: file://a.wasm\n    permissions: {}\n\nstages:\n  release:\n    - run: a.go\n"
    );
  }

  #[test]
  fn add_into_a_file_without_stages_writes_a_plugins_block() {
    let out = add_plugin("name: demo\n", &entry("a", "file://a.wasm")).expect("added");
    assert_eq!(
      out,
      "name: demo\nplugins:\n  - name: a\n    url: file://a.wasm\n    permissions: {}\n"
    );
  }

  #[test]
  fn add_follows_the_indentation_of_existing_items() {
    let input = "plugins:\n- name: a\n  url: file://a.wasm\nstages:\n  s:\n    - run: a.go\n";
    let entry = PluginEntry {
      name: "b".into(),
      url: "file://b.wasm".into(),
      permissions: Permissions {
        exec: vec!["git".into(), "gh".into()],
        ..Permissions::deny()
      },
    };
    let out = add_plugin(input, &entry).expect("added");
    assert_eq!(
      out,
      "plugins:\n- name: a\n  url: file://a.wasm\n- name: b\n  url: file://b.wasm\n  permissions:\n    exec: [\"git\", \"gh\"]\nstages:\n  s:\n    - run: a.go\n"
    );
  }

  fn entry(name: &str, url: &str) -> PluginEntry {
    PluginEntry {
      name: name.into(),
      url: url.into(),
      permissions: Permissions::deny(),
    }
  }

  fn declared(content: &str, name: &str) -> String {
    let config = parse_config(content, SOURCE).expect("valid");
    let plugin = config.plugins.value.iter().find(|p| p.name == name).expect("declared");
    plugin.url.value.as_str().to_string()
  }

  #[test]
  fn add_after_an_item_whose_dash_is_on_its_own_line() {
    let input = "plugins:\n  -\n    name: a\n    url: file://a.wasm\nstages:\n  s:\n    - run: a.go\n";
    let out = add_plugin(input, &entry("b", "file://b.wasm")).expect("added");
    assert_eq!(
      out,
      "plugins:\n  -\n    name: a\n    url: file://a.wasm\n  - name: b\n    url: file://b.wasm\n    permissions: {}\nstages:\n  s:\n    - run: a.go\n"
    );
  }

  #[test]
  fn add_after_an_item_with_a_comment_after_the_dash() {
    let input = "plugins:\n  - # note\n    name: a\n    url: file://a.wasm\nstages:\n  s:\n    - run: a.go\n";
    let out = add_plugin(input, &entry("b", "file://b.wasm")).expect("added");
    assert_eq!(
      out,
      "plugins:\n  - # note\n    name: a\n    url: file://a.wasm\n  - name: b\n    url: file://b.wasm\n    permissions: {}\nstages:\n  s:\n    - run: a.go\n"
    );
  }

  #[test]
  fn set_plugin_on_an_item_whose_dash_is_on_its_own_line() {
    let input = "plugins:\n  -\n    name: a\n    url: file://a.wasm\nstages:\n  s:\n    - run: a.go\n";
    let out = set_plugin(input, "a", "file://a-2.wasm", Some(&Permissions::deny())).expect("set");
    assert_eq!(
      out,
      "plugins:\n  -\n    name: a\n    url: file://a-2.wasm\n    permissions: {}\nstages:\n  s:\n    - run: a.go\n"
    );
  }

  #[test]
  fn add_quotes_values_that_would_not_read_back() {
    let out = add_plugin(RELEASE, &entry("null", "file://a.wasm #frag")).expect("added");
    assert!(
      out.contains("  - name: \"null\"\n    url: \"file://a.wasm #frag\"\n"),
      "{out}"
    );
    assert_eq!(declared(&out, "null"), "file://a.wasm #frag");
    let out = add_plugin(RELEASE, &entry("bad: name", "file://a.wasm")).expect("added");
    assert_eq!(declared(&out, "bad: name"), "file://a.wasm");
  }

  #[test]
  fn set_plugin_quotes_values_that_would_not_read_back() {
    let out = set_plugin(TWO_PLUGINS, "b", "file://b.wasm # x", Some(&Permissions::deny())).expect("set");
    assert_eq!(declared(&out, "b"), "file://b.wasm # x");
  }

  #[test]
  fn add_succeeds_when_a_stage_already_runs_the_new_plugin() {
    let input = "plugins:\n  - name: a\n    url: file://a.wasm\nstages:\n  s:\n    - run: a.go\n    - run: b.go\n";
    let out = add_plugin(input, &entry("b", "file://b.wasm")).expect("added");
    assert_eq!(declared(&out, "b"), "file://b.wasm");
  }

  #[test]
  fn add_still_rejects_a_stage_running_another_undeclared_plugin() {
    let input = "plugins:\n  - name: a\n    url: file://a.wasm\nstages:\n  s:\n    - run: c.go\n";
    let err = add_plugin(input, &entry("b", "file://b.wasm")).expect_err("invalid");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");
  }

  #[test]
  fn add_rejects_an_empty_flow_plugins_list() {
    let input = "plugins: []\nstages:\n  s:\n    - run: b.go\n";
    let err = add_plugin(input, &entry("b", "file://b.wasm")).expect_err("invalid");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");
  }

  fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
  }

  fn has_bare_newline(text: &str) -> bool {
    text.replace("\r\n", "").contains('\n')
  }

  #[test]
  fn add_keeps_crlf_line_endings() {
    let out = add_plugin(&crlf(RELEASE), &github()).expect("added");
    assert_eq!(out, crlf(&added_release()));
    assert!(!has_bare_newline(&out));
  }

  #[test]
  fn set_plugin_keeps_crlf_line_endings() {
    let permissions = Permissions {
      filesystem: FilesystemAccess::ReadWrite,
      ..Permissions::deny()
    };
    let out = set_plugin(&crlf(TWO_PLUGINS), "b", "file://b-2.wasm", Some(&permissions)).expect("set");
    let expected = TWO_PLUGINS.replacen(
      "    url: file://b.wasm\n",
      "    url: file://b-2.wasm\n    permissions:\n      filesystem: read-write\n",
      1,
    );
    assert_eq!(out, crlf(&expected));
    assert!(!has_bare_newline(&out));

    let out = set_plugin(&crlf(TWO_PLUGINS), "a", "file://a-2.wasm", Some(&Permissions::deny())).expect("set");
    assert!(!has_bare_newline(&out), "{out:?}");
    assert_eq!(declared(&out, "a"), "file://a-2.wasm");
  }

  const TWO_PLUGINS: &str = "plugins:
  - name: a
    url: 'file://a-1.wasm'  # pinned
    # grants below
    permissions:
      network: [old.example.com]  # legacy
      filesystem: read-write
    config:
      level: 3
  - name: b
    url: file://b.wasm
# trailing comment
stages:
  s:
    - run: a.go
    - run: b.go
";

  #[test]
  fn set_plugin_changes_only_url_and_permissions() {
    let permissions = Permissions {
      network: vec!["api.example.com".into(), "*.acme.io".into()],
      ..Permissions::deny()
    };
    let out = set_plugin(TWO_PLUGINS, "a", "file://a-2.wasm", Some(&permissions)).expect("set");
    let expected = TWO_PLUGINS.replacen("'file://a-1.wasm'", "file://a-2.wasm", 1).replacen(
      "network: [old.example.com]  # legacy\n      filesystem: read-write",
      "network: [\"api.example.com\", \"*.acme.io\"]",
      1,
    );
    assert_eq!(out, expected);
  }

  #[test]
  fn set_plugin_with_the_same_url_leaves_the_url_untouched() {
    let permissions = Permissions {
      network: vec!["api.example.com".into()],
      ..Permissions::deny()
    };
    let out = set_plugin(TWO_PLUGINS, "a", "file://a-1.wasm", Some(&permissions)).expect("set");
    let expected = TWO_PLUGINS.replacen(
      "network: [old.example.com]  # legacy\n      filesystem: read-write",
      "network: [\"api.example.com\"]",
      1,
    );
    assert_eq!(out, expected);
  }

  #[test]
  fn set_plugin_inserts_permissions_when_absent() {
    let permissions = Permissions {
      filesystem: FilesystemAccess::ReadWrite,
      ..Permissions::deny()
    };
    let out = set_plugin(TWO_PLUGINS, "b", "file://b-2.wasm", Some(&permissions)).expect("set");
    let expected = TWO_PLUGINS.replacen(
      "    url: file://b.wasm\n",
      "    url: file://b-2.wasm\n    permissions:\n      filesystem: read-write\n",
      1,
    );
    assert_eq!(out, expected);
  }

  #[test]
  fn set_plugin_rejects_unknown_name() {
    let err = set_plugin(TWO_PLUGINS, "zz", "file://z.wasm", Some(&Permissions::deny())).expect_err("missing");
    assert!(matches!(err, ReleaseFileError::PluginNotFound(ref n) if n == "zz"), "{err:?}");
  }

  fn config_text(content: &str, plugin: &str, key: &str) -> String {
    let config = parse_config_unvalidated(content, SOURCE).expect("parses");
    let plugin = config.plugins.value.iter().find(|p| p.name == plugin).expect("declared");
    match &plugin.config.get(key).expect("key").value {
      moonlit_engine::pipeline::config::ConfigValue::String(text) => text.clone(),
      other => panic!("not a string: {other:?}"),
    }
  }

  const KEPT_LAST: &str = "name: d\nstages:\n  s:\n    - run: a.go\nplugins:\n  - name: a\n    url: file://a.wasm\n    config:\n      notes: |+\n        line one\n\n\n";

  #[test]
  fn add_after_a_keep_chomped_scalar_keeps_its_trailing_lines() {
    assert_eq!(config_text(KEPT_LAST, "a", "notes"), "line one\n\n\n");
    let out = add_plugin(KEPT_LAST, &entry("b", "file://b.wasm")).expect("added");
    assert_eq!(config_text(&out, "a", "notes"), "line one\n\n\n");
    assert_eq!(declared(&out, "b"), "file://b.wasm");

    let folded = crlf(&KEPT_LAST.replace("|+", ">+"));
    let out = add_plugin(&folded, &entry("b", "file://b.wasm")).expect("added");
    assert_eq!(config_text(&out, "a", "notes"), "line one\n\n\n");
  }

  #[test]
  fn add_after_an_empty_keep_chomped_scalar_keeps_its_value() {
    let input = "name: d\nstages:\n  s:\n    - run: a.go\nplugins:\n  - name: a\n    url: file://a.wasm\n    config:\n      notes: |+\n\n\n";
    let before = config_text(input, "a", "notes");
    let out = add_plugin(input, &entry("b", "file://b.wasm")).expect("added");
    assert_eq!(config_text(&out, "a", "notes"), before);
    assert_eq!(declared(&out, "b"), "file://b.wasm");
  }

  #[test]
  fn set_plugin_inserting_permissions_after_a_keep_chomped_scalar_keeps_it() {
    let input = "plugins:\n  - name: a\n    url: file://a.wasm\n    config:\n      notes: |+\n        line one\n\n\nstages:\n  s:\n    - run: a.go\n";
    let permissions = Permissions {
      exec: vec!["git".into()],
      ..Permissions::deny()
    };
    let out = set_plugin(input, "a", "file://a.wasm", Some(&permissions)).expect("set");
    assert_eq!(config_text(&out, "a", "notes"), "line one\n\n\n");
    let config = parse_config(&out, SOURCE).expect("valid");
    assert_eq!(config.plugins.value[0].permissions.as_ref(), Some(&permissions));
  }

  #[test]
  fn read_back_rejects_a_change_outside_the_edited_entry() {
    let url = "file://b-2.wasm";
    let after = set_plugin(TWO_PLUGINS, "b", url, None).expect("set");
    assert!(validated(TWO_PLUGINS, after.clone(), "b", url, None).is_ok());

    let other_config = after.replace("level: 3", "level: 4");
    let err = validated(TWO_PLUGINS, other_config, "b", url, None).expect_err("a's config changed");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");

    let stages = after.replace("    - run: a.go\n", "");
    let err = validated(TWO_PLUGINS, stages, "b", url, None).expect_err("stages changed");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");

    let own_config = after.replace(
      "    url: file://b-2.wasm\n",
      "    url: file://b-2.wasm\n    config:\n      x: 1\n",
    );
    let err = validated(TWO_PLUGINS, own_config, "b", url, None).expect_err("b's config changed");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");
  }

  #[test]
  fn edits_succeed_for_plugins_named_like_numbers_or_booleans() {
    for name in ["1.10", "True", "0x1f"] {
      let out = add_plugin(RELEASE, &entry(name, "file://n.wasm")).unwrap_or_else(|e| panic!("add {name}: {e}"));
      assert_eq!(declared(&out, name), "file://n.wasm", "add {name}");
      let out = set_plugin(&out, name, "file://n-2.wasm", None).unwrap_or_else(|e| panic!("set {name}: {e}"));
      assert_eq!(declared(&out, name), "file://n-2.wasm", "set {name}");
    }
  }

  #[test]
  fn diff_shows_only_added_lines() {
    let diff = render_diff(RELEASE, &added_release());
    assert_eq!(
      diff,
      "  +   - name: github\n  +     url: oci://registry.moonlit.rs/moonlit/github:2.0.0\n  +     permissions:\n  +       network: [\"api.github.com\"]\n  +       env: [\"GITHUB_TOKEN\"]\n  +       filesystem: read-only\n"
    );
  }

  #[test]
  fn diff_shows_removed_then_added_lines() {
    let diff = render_diff("a\nb\nc\n", "a\nB\nc\n");
    assert_eq!(diff, "  - b\n  + B\n");
  }
}
