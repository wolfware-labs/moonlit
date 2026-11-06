use moonlit_engine::pipeline::config::{FilesystemAccess, Permissions, parse_config};

/// A plugin entry to write into a release file.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginEntry {
  pub name: String,
  pub url: String,
  pub permissions: Permissions,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ReleaseFileError {
  #[error("a plugin named '{0}' is already declared")]
  #[diagnostic(code(moonlit::release_file::name_taken))]
  NameTaken(String),
  #[error("this plugin is already declared as '{name}'")]
  #[diagnostic(code(moonlit::release_file::url_present))]
  UrlPresent { name: String },
  #[error("no plugin named '{0}' is declared")]
  #[diagnostic(code(moonlit::release_file::plugin_not_found))]
  PluginNotFound(String),
  #[error("the edited release file would be invalid: {0}")]
  #[diagnostic(code(moonlit::release_file::invalid))]
  Invalid(String),
}

/// Appends a plugin entry to the `plugins:` sequence, leaving every other byte of the file unchanged.
pub fn add_plugin(content: &str, entry: &PluginEntry) -> Result<String, ReleaseFileError> {
  let edit = match parse_config(content, SOURCE) {
    Ok(config) => {
      let plugins = &config.plugins.value;
      if plugins.iter().any(|p| p.name == entry.name) {
        return Err(ReleaseFileError::NameTaken(entry.name.clone()));
      }
      if let Some(p) = plugins.iter().find(|p| p.url.value.as_str() == entry.url) {
        return Err(ReleaseFileError::UrlPresent { name: p.name.clone() });
      }
      let Some(last) = plugins.last() else {
        return Err(ReleaseFileError::Invalid("no plugins are declared".into()));
      };
      let dash = column(content, last.span.start);
      let key = key_column(content, last.span.start);
      insertion(content, line_end(content, last.span.end), item_block(entry, dash, key))
    }
    Err(err) if top_level_line(content, "plugins:").is_some() => return Err(ReleaseFileError::Invalid(err.message().into())),
    Err(_) => {
      let at = top_level_line(content, "stages:").unwrap_or(content.len());
      insertion(content, at, format!("plugins:\n{}\n", item_block(entry, 2, 4)))
    }
  };
  validated(apply(content, vec![edit]))
}

/// Replaces the `url` and `permissions` of one plugin entry, leaving every other byte of the file unchanged.
pub fn set_plugin(content: &str, name: &str, url: &str, permissions: &Permissions) -> Result<String, ReleaseFileError> {
  let config = parse_config(content, SOURCE).map_err(|err| ReleaseFileError::Invalid(err.message().into()))?;
  let plugin = config
    .plugins
    .value
    .iter()
    .find(|p| p.name == name)
    .ok_or_else(|| ReleaseFileError::PluginNotFound(name.to_string()))?;
  let url_edit = (plugin.url.span.start, plugin.url.span.end, url.to_string());
  let permissions_edit = if let Some(span) = plugin.permissions_span {
    let block = permissions_block(permissions, column(content, span.start));
    (span.start, span.end, block)
  } else {
    let key = key_column(content, plugin.span.start);
    let lines = format!("{}{}\n", " ".repeat(key), permissions_block(permissions, key));
    insertion(content, line_end(content, plugin.span.end), lines)
  };
  validated(apply(content, vec![url_edit, permissions_edit]))
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

fn validated(content: String) -> Result<String, ReleaseFileError> {
  match parse_config(&content, SOURCE) {
    Ok(_) => Ok(content),
    Err(err) => Err(ReleaseFileError::Invalid(err.message().into())),
  }
}

#[must_use]
fn apply(content: &str, mut edits: Vec<Edit>) -> String {
  edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
  let mut out = content.to_string();
  for (start, end, text) in edits {
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
  match content[item_start..].strip_prefix('-') {
    Some(rest) => col + 1 + (rest.len() - rest.trim_start_matches(' ').len()),
    None => col,
  }
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
  let gap = " ".repeat(key - dash - 1);
  let pad = " ".repeat(key);
  format!(
    "{}-{gap}name: {}\n{pad}url: {}\n{pad}{}\n",
    " ".repeat(dash),
    entry.name,
    entry.url,
    permissions_block(&entry.permissions, key)
  )
}

#[must_use]
fn permissions_block(permissions: &Permissions, key: usize) -> String {
  let mut lines = Vec::new();
  for (kind, values) in [
    ("network", &permissions.network),
    ("exec", &permissions.exec),
    ("env", &permissions.env),
  ] {
    if !values.is_empty() {
      let quoted: Vec<String> = values.iter().map(|v| quote(v)).collect();
      lines.push(format!("{kind}: [{}]", quoted.join(", ")));
    }
  }
  match permissions.filesystem {
    FilesystemAccess::None => {}
    FilesystemAccess::ReadOnly => lines.push("filesystem: read-only".into()),
    FilesystemAccess::ReadWrite => lines.push("filesystem: read-write".into()),
  }
  if lines.is_empty() {
    return "permissions: {}".into();
  }
  let pad = " ".repeat(key + 2);
  let body: Vec<String> = lines.iter().map(|line| format!("{pad}{line}")).collect();
  format!("permissions:\n{}", body.join("\n"))
}

#[must_use]
fn quote(value: &str) -> String {
  format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
  use super::*;

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
    assert!(matches!(err, ReleaseFileError::NameTaken(ref n) if n == "git"), "{err:?}");
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

  #[test]
  fn add_refuses_to_write_an_invalid_file() {
    let mut entry = github();
    entry.name = "bad: name".into();
    let err = add_plugin(RELEASE, &entry).expect_err("invalid");
    assert!(matches!(err, ReleaseFileError::Invalid(_)), "{err:?}");
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
    let out = set_plugin(TWO_PLUGINS, "a", "file://a-2.wasm", &permissions).expect("set");
    let expected = TWO_PLUGINS.replacen("'file://a-1.wasm'", "file://a-2.wasm", 1).replacen(
      "network: [old.example.com]  # legacy\n      filesystem: read-write",
      "network: [\"api.example.com\", \"*.acme.io\"]",
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
    let out = set_plugin(TWO_PLUGINS, "b", "file://b-2.wasm", &permissions).expect("set");
    let expected = TWO_PLUGINS.replacen(
      "    url: file://b.wasm\n",
      "    url: file://b-2.wasm\n    permissions:\n      filesystem: read-write\n",
      1,
    );
    assert_eq!(out, expected);
  }

  #[test]
  fn set_plugin_rejects_unknown_name() {
    let err = set_plugin(TWO_PLUGINS, "zz", "file://z.wasm", &Permissions::deny()).expect_err("missing");
    assert!(matches!(err, ReleaseFileError::PluginNotFound(ref n) if n == "zz"), "{err:?}");
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
