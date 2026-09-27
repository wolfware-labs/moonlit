use crate::pipeline::config::model::{ConfigValue, Plugin, PluginUrl, Run, Stage, Step};
use crate::pipeline::config::source::ConfigSource;
use crate::pipeline::config::span::{Span, Spanned};
use crate::pipeline::config::tree::{Node, NodeValue};
use crate::pipeline::config::{ConfigDiagnostic, ConfigMap, FilesystemAccess, Permissions, PipelineConfig, tree};
use indexmap::IndexMap;

pub fn parse_config(yaml: &str, source_name: &str) -> Result<PipelineConfig, ConfigDiagnostic> {
  let src = ConfigSource::new(yaml, source_name);
  let tree = tree::build_tree(&src)?;
  let config = convert(&tree, &src)?;
  let config = cleanup(config);
  validate(&config, &src)?;
  Ok(config)
}

fn convert(root: &Node, src: &ConfigSource) -> Result<PipelineConfig, ConfigDiagnostic> {
  match &root.value {
    NodeValue::Null => Ok(create_empty(root.span)),
    NodeValue::Map(entries) => convert_root(entries, root.span, src),
    _ => Err(ConfigDiagnostic::expected_mapping(
      src,
      "the release configuration",
      root.span,
    )),
  }
}

#[must_use]
fn cleanup(mut config: PipelineConfig) -> PipelineConfig {
  config.name = config.name.trim().to_string();
  config
}

fn validate(config: &PipelineConfig, src: &ConfigSource) -> Result<(), ConfigDiagnostic> {
  if config.stages.value.is_empty() {
    return Err(ConfigDiagnostic::no_stages(src));
  }
  if config.plugins.value.is_empty() {
    return Err(ConfigDiagnostic::no_plugins(src, Some(config.plugins.span)));
  }
  let mut seen = std::collections::HashSet::new();
  for plugin in &config.plugins.value {
    if !seen.insert(plugin.name.as_str()) {
      return Err(ConfigDiagnostic::duplicate_plugin(src, &plugin.name, config.plugins.span));
    }
  }
  for step in config.stages.value.iter().flat_map(|stage| &stage.steps) {
    let plugin = &step.run.value.plugin;
    if !seen.contains(plugin.as_str()) {
      return Err(ConfigDiagnostic::plugin_not_found(src, plugin, step.run.span));
    }
  }
  Ok(())
}

#[must_use]
fn create_empty(span: Span) -> PipelineConfig {
  PipelineConfig {
    name: String::new(),
    arguments: IndexMap::new(),
    variables: IndexMap::new(),
    plugins: Spanned::new(Vec::new(), span),
    stages: Spanned::new(Vec::new(), span),
  }
}

fn convert_root(entries: &[(Node, Node)], root_span: Span, src: &ConfigSource) -> Result<PipelineConfig, ConfigDiagnostic> {
  let mut name: Option<String> = None;
  let mut arguments: Option<IndexMap<String, String>> = None;
  let mut variables: Option<IndexMap<String, String>> = None;
  let mut plugins: Option<Spanned<Vec<Plugin>>> = None;
  let mut stages: Option<Spanned<Vec<Stage>>> = None;

  let mut seen: Vec<&str> = Vec::new();
  for (key, value) in entries {
    if let Some(k) = schema_key(key) {
      if seen.contains(&k) {
        return Err(ConfigDiagnostic::duplicate_key(src, k, key.span));
      }
      seen.push(k);
    }
    match schema_key(key) {
      Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
      Some("arguments") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(src, "arguments", "a mapping", value.span));
        }
        arguments = Some(string_map(value, "arguments", src)?);
      }
      Some("variables") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(src, "variables", "a mapping", value.span));
        }
        variables = Some(string_map(value, "variables", src)?);
      }
      Some("plugins") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(
            src,
            "plugins",
            "a sequence of plugins",
            value.span,
          ));
        }
        plugins = Some(convert_plugins(value, src)?);
      }
      Some("stages") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(src, "stages", "a mapping of stages", value.span));
        }
        stages = Some(convert_stages(value, src)?);
      }
      other => {
        return Err(ConfigDiagnostic::unknown_key(
          src,
          other.unwrap_or_default(),
          "configuration",
          key.span,
        ));
      }
    }
  }

  Ok(PipelineConfig {
    name: name.unwrap_or_default(),
    arguments: arguments.unwrap_or_default(),
    variables: variables.unwrap_or_default(),
    plugins: plugins.unwrap_or_else(|| Spanned::new(Vec::new(), root_span)),
    stages: stages.unwrap_or_else(|| Spanned::new(Vec::new(), root_span)),
  })
}

#[must_use]
fn schema_key(key: &Node) -> Option<&str> {
  match &key.value {
    NodeValue::Scalar(raw) => Some(raw.as_str()),
    _ => None,
  }
}

#[must_use]
fn raw_key(key: &Node) -> Option<&str> {
  match &key.value {
    NodeValue::Scalar(raw) => Some(raw.as_str()),
    _ => None,
  }
}

#[must_use]
fn scalar_string(node: &Node) -> Option<String> {
  match &node.value {
    NodeValue::Scalar(raw) => Some(raw.clone()),
    _ => None,
  }
}

fn string_map(node: &Node, context: &str, src: &ConfigSource) -> Result<IndexMap<String, String>, ConfigDiagnostic> {
  let mut out = IndexMap::new();
  if let NodeValue::Map(entries) = &node.value {
    for (key, value) in entries {
      let Some(k) = raw_key(key) else { continue };
      match &value.value {
        NodeValue::Null => {} // filtered
        NodeValue::Scalar(raw) => {
          out.insert(k.to_string(), raw.clone());
        }
        _ => return Err(ConfigDiagnostic::expected_string(src, context, value.span)),
      }
    }
  }
  Ok(out)
}

fn convert_plugins(node: &Node, src: &ConfigSource) -> Result<Spanned<Vec<Plugin>>, ConfigDiagnostic> {
  let mut plugins = Vec::new();
  if let NodeValue::Seq(items) = &node.value {
    for item in items {
      plugins.push(convert_plugin(item, src)?);
    }
  }
  Ok(Spanned::new(plugins, node.span))
}

fn convert_plugin(node: &Node, src: &ConfigSource) -> Result<Plugin, ConfigDiagnostic> {
  let NodeValue::Map(entries) = &node.value else {
    return Err(ConfigDiagnostic::expected_mapping(src, "a plugin", node.span));
  };
  let mut name: Option<String> = None;
  let mut url: Option<Spanned<PluginUrl>> = None;
  let mut config: Option<ConfigMap> = None;
  let mut permissions: Option<Permissions> = None;

  let mut seen: Vec<&str> = Vec::new();
  for (key, value) in entries {
    if let Some(k) = schema_key(key) {
      if seen.contains(&k) {
        return Err(ConfigDiagnostic::duplicate_key(src, k, key.span));
      }
      seen.push(k);
    }
    match schema_key(key) {
      Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
      Some("url") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(src, "url", "a plugin URL", value.span));
        }
        url = Some(convert_url(value, src)?);
      }
      Some("config") => config = Some(config_map(value)),
      Some("permissions") => permissions = Some(convert_permissions(value, src)?),
      other => {
        return Err(ConfigDiagnostic::unknown_key(
          src,
          other.unwrap_or_default(),
          "plugin",
          key.span,
        ));
      }
    }
  }

  let name = name.unwrap_or_default();
  let Some(url) = url else {
    return Err(ConfigDiagnostic::missing_url(src, &name, node.span));
  };
  Ok(Plugin {
    name,
    url,
    config: config.unwrap_or_default(),
    permissions,
  })
}

fn convert_url(node: &Node, src: &ConfigSource) -> Result<Spanned<PluginUrl>, ConfigDiagnostic> {
  let raw = scalar_string(node).ok_or_else(|| ConfigDiagnostic::invalid_url(src, "", node.span))?;
  let scheme = raw.split_once("://").map(|(s, _)| s.to_lowercase());
  let url = match scheme.as_deref() {
    Some("oci") => PluginUrl::Oci(raw.clone()),
    Some("file") => PluginUrl::File(raw.clone()),
    Some("http") => PluginUrl::Http(raw.clone()),
    Some("https") => PluginUrl::Https(raw.clone()),
    _ => return Err(ConfigDiagnostic::invalid_url(src, &raw, node.span)),
  };
  Ok(Spanned::new(url, node.span))
}

#[must_use]
fn config_map(node: &Node) -> ConfigMap {
  match to_config_value(node).value {
    ConfigValue::Map(m) => m,
    _ => ConfigMap::new(),
  }
}

#[must_use]
fn to_config_value(node: &Node) -> Spanned<ConfigValue> {
  let value = match &node.value {
    NodeValue::Null => ConfigValue::Null,
    NodeValue::Scalar(raw) => ConfigValue::String(raw.clone()),
    NodeValue::Seq(items) => ConfigValue::List(items.iter().map(to_config_value).collect()),
    NodeValue::Map(entries) => {
      let mut m = ConfigMap::new();
      for (key, val) in entries {
        if let NodeValue::Scalar(raw) = &key.value {
          m.insert(raw.clone(), to_config_value(val));
        }
      }
      ConfigValue::Map(m)
    }
  };
  Spanned::new(value, node.span)
}

fn convert_permissions(node: &Node, src: &ConfigSource) -> Result<Permissions, ConfigDiagnostic> {
  let NodeValue::Map(_) = &node.value else {
    return Err(ConfigDiagnostic::expected_mapping(src, "a plugin's permissions", node.span));
  };
  let mut p = Permissions::deny();
  if let NodeValue::Map(entries) = &node.value {
    let mut seen: Vec<&str> = Vec::new();
    for (key, value) in entries {
      if let Some(k) = schema_key(key) {
        if seen.contains(&k) {
          return Err(ConfigDiagnostic::duplicate_key(src, k, key.span));
        }
        seen.push(k);
      }
      match schema_key(key) {
        Some("network") => p.network = string_list(value),
        Some("exec") => p.exec = string_list(value),
        Some("env") => p.env = string_list(value),
        Some("filesystem") => {
          if let Some(s) = scalar_string(value) {
            p.filesystem = parse_fs(&s).ok_or_else(|| ConfigDiagnostic::invalid_filesystem(src, &s, value.span))?;
          }
        }
        other => {
          return Err(ConfigDiagnostic::unknown_key(
            src,
            other.unwrap_or_default(),
            "permissions",
            key.span,
          ));
        }
      }
    }
  }
  Ok(p)
}

#[must_use]
fn string_list(node: &Node) -> Vec<String> {
  match &node.value {
    NodeValue::Seq(items) => items.iter().filter_map(scalar_string).collect(),
    NodeValue::Scalar(raw) => vec![raw.clone()],
    _ => Vec::new(),
  }
}

#[must_use]
fn parse_fs(raw: &str) -> Option<FilesystemAccess> {
  match raw.to_lowercase().as_str() {
    "none" => Some(FilesystemAccess::None),
    "read-only" | "readonly" => Some(FilesystemAccess::ReadOnly),
    "read-write" | "readwrite" => Some(FilesystemAccess::ReadWrite),
    _ => None,
  }
}

fn convert_stages(node: &Node, src: &ConfigSource) -> Result<Spanned<Vec<Stage>>, ConfigDiagnostic> {
  let mut stages = Vec::new();
  if let NodeValue::Map(entries) = &node.value {
    for (key, value) in entries {
      let Some(stage_name) = raw_key(key) else {
        continue;
      };
      if matches!(value.value, NodeValue::Null) {
        return Err(ConfigDiagnostic::null_value(
          src,
          stage_name,
          "a sequence of steps",
          value.span,
        ));
      }
      let steps = convert_steps(value, src)?;
      stages.push(Stage {
        name: stage_name.to_string(),
        steps,
      });
    }
  }
  Ok(Spanned::new(stages, node.span))
}

fn convert_steps(node: &Node, src: &ConfigSource) -> Result<Vec<Step>, ConfigDiagnostic> {
  let NodeValue::Seq(items) = &node.value else {
    return Err(ConfigDiagnostic::expected_sequence(src, "a stage's steps", node.span));
  };
  let mut steps = Vec::new();
  for item in items {
    steps.push(convert_step(item, src)?);
  }
  Ok(steps)
}

fn convert_step(node: &Node, src: &ConfigSource) -> Result<Step, ConfigDiagnostic> {
  let NodeValue::Map(entries) = &node.value else {
    return Err(ConfigDiagnostic::expected_mapping(src, "a step", node.span));
  };
  let mut name: Option<String> = None;
  let mut run: Option<Spanned<Run>> = None;
  let mut condition: Option<String> = None;
  let mut halt_if: Option<String> = None;
  let mut continue_on_error: Option<bool> = None;
  let mut config: Option<ConfigMap> = None;

  let mut seen: Vec<&str> = Vec::new();
  for (key, value) in entries {
    if let Some(k) = schema_key(key) {
      if seen.contains(&k) {
        return Err(ConfigDiagnostic::duplicate_key(src, k, key.span));
      }
      seen.push(k);
    }
    match schema_key(key) {
      Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
      Some("run") => run = Some(convert_run(value, src)?),
      Some("condition") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(src, "condition", "an expression", value.span));
        }
        condition = scalar_string(value);
      }
      Some("haltIf") => {
        if matches!(value.value, NodeValue::Null) {
          return Err(ConfigDiagnostic::null_value(src, "haltIf", "an expression", value.span));
        }
        halt_if = scalar_string(value);
      }
      Some("continueOnError") => continue_on_error = Some(parse_bool(value, src)?),
      Some("config") => config = Some(config_map(value)),
      other => {
        return Err(ConfigDiagnostic::unknown_key(
          src,
          other.unwrap_or_default(),
          "step",
          key.span,
        ));
      }
    }
  }

  let name = name.unwrap_or_default();
  let Some(run) = run else {
    return Err(ConfigDiagnostic::missing_run(src, &name, node.span));
  };
  Ok(Step {
    name,
    run,
    condition,
    halt_if,
    continue_on_error: continue_on_error.unwrap_or(false),
    config: config.unwrap_or_default(),
  })
}

fn convert_run(node: &Node, src: &ConfigSource) -> Result<Spanned<Run>, ConfigDiagnostic> {
  let raw = scalar_string(node).unwrap_or_default();
  match raw.split_once('.') {
    Some((plugin, middleware)) if !plugin.is_empty() && !middleware.is_empty() => Ok(Spanned::new(
      Run {
        plugin: plugin.to_string(),
        middleware: middleware.to_string(),
      },
      node.span,
    )),
    _ => Err(ConfigDiagnostic::invalid_run(src, &raw, node.span)),
  }
}

fn parse_bool(node: &Node, src: &ConfigSource) -> Result<bool, ConfigDiagnostic> {
  let raw = scalar_string(node).unwrap_or_default();
  match raw.to_lowercase().as_str() {
    "true" => Ok(true),
    "false" => Ok(false),
    _ => Err(ConfigDiagnostic::invalid_bool(src, "continueOnError", &raw, node.span)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use miette::Diagnostic;

  const PLUGINS: &str = "plugins:\n  - name: tp\n    url: file://tp.wasm\n";
  const STAGES: &str = "stages:\n  build:\n    - name: s1\n      run: tp.go\n";

  fn parse(yaml: &str) -> PipelineConfig {
    parse_config(yaml, "release.yml").expect("valid configuration")
  }

  fn error(yaml: &str) -> ConfigDiagnostic {
    parse_config(yaml, "release.yml").expect_err("invalid configuration")
  }

  fn underlined<'a>(yaml: &'a str, diagnostic: &ConfigDiagnostic) -> &'a str {
    let span = diagnostic.span().expect("diagnostic carries a span");
    &yaml[span.offset()..span.offset() + span.len()]
  }

  fn label(diagnostic: &ConfigDiagnostic) -> String {
    diagnostic
      .labels()
      .and_then(|mut labels| labels.next())
      .and_then(|label| label.label().map(str::to_string))
      .expect("diagnostic carries a label")
  }

  fn with_plugins_and_stages(extra: &str) -> String {
    format!("{extra}{PLUGINS}{STAGES}")
  }

  fn step_yaml(step_body: &str) -> String {
    format!("{PLUGINS}stages:\n  build:\n    - name: s1\n      run: tp.go\n{step_body}")
  }

  fn plugin_yaml(plugin_body: &str) -> String {
    format!("plugins:\n  - name: tp\n    url: file://tp.wasm\n{plugin_body}{STAGES}")
  }

  #[test]
  fn parses_every_top_level_section() {
    let yaml = "name: '  demo  '\narguments:\n  version: '1.0'\n  skipped: ~\nvariables:\n  region: eu\n";
    let config = parse(&with_plugins_and_stages(yaml));

    assert_eq!(config.name, "demo");
    assert_eq!(config.arguments.get("version").map(String::as_str), Some("1.0"));
    assert!(!config.arguments.contains_key("skipped"));
    assert_eq!(config.variables.get("region").map(String::as_str), Some("eu"));
    assert_eq!(config.plugins.value.len(), 1);
    assert_eq!(config.stages.value[0].name, "build");
  }

  #[test]
  fn parses_plugin_urls_by_scheme() {
    let yaml = "plugins:\n  - name: a\n    url: oci://reg/a:1\n  - name: b\n    url: file://b.wasm\n  - name: c\n    url: http://host/c.wasm\n  - name: d\n    url: HTTPS://host/d.wasm\nstages:\n  build:\n    - run: a.go\n";
    let config = parse(yaml);
    let urls: Vec<_> = config.plugins.value.iter().map(|p| p.url.value.clone()).collect();

    assert_eq!(
      urls,
      vec![
        PluginUrl::Oci("oci://reg/a:1".into()),
        PluginUrl::File("file://b.wasm".into()),
        PluginUrl::Http("http://host/c.wasm".into()),
        PluginUrl::Https("HTTPS://host/d.wasm".into()),
      ]
    );
  }

  #[test]
  fn parses_plugin_config_into_a_nested_value_tree() {
    let yaml = plugin_yaml(
      "    config:\n      token: abc\n      list: [x, ~]\n      nested:\n        deep: 1\n      empty: ~\n      ? [ignored]\n      : dropped\n",
    );
    let config = parse(&yaml);
    let plugin_config = &config.plugins.value[0].config;

    let values = |value: &ConfigValue| match value {
      ConfigValue::List(items) => items.iter().map(|item| item.value.clone()).collect(),
      ConfigValue::Map(entries) => entries.values().map(|entry| entry.value.clone()).collect(),
      other => vec![other.clone()],
    };

    assert_eq!(values(&plugin_config["token"].value), vec![ConfigValue::String("abc".into())]);
    assert_eq!(
      values(&plugin_config["list"].value),
      vec![ConfigValue::String("x".into()), ConfigValue::Null]
    );
    assert_eq!(values(&plugin_config["nested"].value), vec![ConfigValue::String("1".into())]);
    assert_eq!(plugin_config["empty"].value, ConfigValue::Null);
    assert_eq!(plugin_config.len(), 4);
  }

  #[test]
  fn non_mapping_plugin_config_becomes_empty() {
    let config = parse(&plugin_yaml("    config: just-a-string\n"));
    assert!(config.plugins.value[0].config.is_empty());
  }

  #[test]
  fn plugin_without_permissions_leaves_them_unset() {
    let config = parse(&with_plugins_and_stages(""));
    assert_eq!(config.plugins.value[0].permissions, None);
  }

  #[test]
  fn parses_permissions_lists_and_filesystem_levels() {
    let yaml = plugin_yaml(
      "    permissions:\n      network: [api.example.com, '*.acme.io']\n      exec: git\n      env: {}\n      filesystem: Read-Only\n",
    );
    let permissions = parse(&yaml).plugins.value[0].permissions.clone().expect("permissions");

    assert_eq!(permissions.network, vec!["api.example.com", "*.acme.io"]);
    assert_eq!(permissions.exec, vec!["git"]);
    assert!(permissions.env.is_empty());
    assert_eq!(permissions.filesystem, FilesystemAccess::ReadOnly);
  }

  #[test]
  fn filesystem_accepts_every_spelling() {
    for (raw, expected) in [
      ("none", FilesystemAccess::None),
      ("readonly", FilesystemAccess::ReadOnly),
      ("read-only", FilesystemAccess::ReadOnly),
      ("readwrite", FilesystemAccess::ReadWrite),
      ("READ-WRITE", FilesystemAccess::ReadWrite),
    ] {
      let yaml = plugin_yaml(&format!("    permissions:\n      filesystem: {raw}\n"));
      let permissions = parse(&yaml).plugins.value[0].permissions.clone().expect("permissions");
      assert_eq!(permissions.filesystem, expected, "{raw}");
    }
  }

  #[test]
  fn non_scalar_filesystem_keeps_the_deny_default() {
    let yaml = plugin_yaml("    permissions:\n      filesystem: [read-write]\n");
    let permissions = parse(&yaml).plugins.value[0].permissions.clone().expect("permissions");
    assert_eq!(permissions.filesystem, FilesystemAccess::None);
  }

  #[test]
  fn parses_every_step_field() {
    let yaml = step_yaml(
      "      condition: args.version != ''\n      haltIf: output.s1.done == 'true'\n      continueOnError: 'TRUE'\n      config:\n        level: high\n    - name: s2\n      run: tp.other\n      continueOnError: false\n",
    );
    let config = parse(&yaml);
    let steps = &config.stages.value[0].steps;

    assert_eq!(steps[0].name, "s1");
    assert_eq!(steps[0].run.value.plugin, "tp");
    assert_eq!(steps[0].run.value.middleware, "go");
    assert_eq!(steps[0].condition.as_deref(), Some("args.version != ''"));
    assert_eq!(steps[0].halt_if.as_deref(), Some("output.s1.done == 'true'"));
    assert!(steps[0].continue_on_error);
    assert_eq!(steps[0].config["level"].value, ConfigValue::String("high".into()));
    assert!(!steps[1].continue_on_error);
    assert_eq!(steps[1].condition, None);
  }

  #[test]
  fn step_without_name_gets_an_empty_name() {
    let config = parse(&format!("{PLUGINS}stages:\n  build:\n    - run: tp.go\n"));
    assert_eq!(config.stages.value[0].steps[0].name, "");
  }

  #[test]
  fn keeps_stage_order_and_skips_non_scalar_stage_keys() {
    let yaml = format!("{PLUGINS}stages:\n  deploy:\n    - run: tp.a\n  ? [odd]\n  : - run: tp.b\n  build:\n    - run: tp.c\n");
    let names: Vec<_> = parse(&yaml).stages.value.iter().map(|s| s.name.clone()).collect();
    assert_eq!(names, vec!["deploy", "build"]);
  }

  #[test]
  fn non_scalar_argument_keys_are_skipped() {
    let config = parse(&with_plugins_and_stages("arguments:\n  ? [odd]\n  : v\n  kept: yes\n"));
    assert_eq!(config.arguments.len(), 1);
  }

  #[test]
  fn non_mapping_arguments_are_ignored() {
    let config = parse(&with_plugins_and_stages("arguments: scalar\n"));
    assert!(config.arguments.is_empty());
  }

  #[test]
  fn quoted_null_stays_a_string_and_tilde_is_null() {
    let config = parse(&with_plugins_and_stages(
      "variables:\n  quoted: 'null'\n  tilde: ~\n  word: NULL\n",
    ));
    assert_eq!(config.variables.get("quoted").map(String::as_str), Some("null"));
    assert!(!config.variables.contains_key("tilde"));
    assert!(!config.variables.contains_key("word"));
  }

  #[test]
  fn aliases_reuse_the_anchored_value() {
    let yaml = format!(
      "variables: &shared\n  region: eu\narguments: *shared\n{PLUGINS}stages:\n  build: &steps\n    - run: tp.go\n  again: *steps\n"
    );
    let config = parse(&yaml);
    assert_eq!(config.arguments.get("region").map(String::as_str), Some("eu"));
    assert_eq!(config.stages.value[1].steps[0].run.value.middleware, "go");
  }

  #[test]
  fn anchored_scalar_can_be_aliased() {
    let config = parse(&with_plugins_and_stages("name: &n demo\nvariables:\n  copy: *n\n"));
    assert_eq!(config.variables.get("copy").map(String::as_str), Some("demo"));
  }

  #[test]
  fn document_markers_are_accepted() {
    let config = parse(&format!("---\n{PLUGINS}{STAGES}...\n"));
    assert_eq!(config.stages.value.len(), 1);
  }

  #[test]
  fn empty_document_has_no_stages() {
    let diagnostic = error("");
    assert_eq!(
      diagnostic.message(),
      "No stages defined. A pipeline needs at least one stage."
    );
    assert!(diagnostic.span().is_none());
  }

  #[test]
  fn null_document_has_no_stages() {
    let diagnostic = error("~\n");
    assert_eq!(
      diagnostic.message(),
      "No stages defined. A pipeline needs at least one stage."
    );
  }

  #[test]
  fn root_must_be_a_mapping() {
    let yaml = "- a\n- b\n";
    let diagnostic = error(yaml);
    assert_eq!(diagnostic.message(), "Expected a mapping for the release configuration.");
    assert_eq!(label(&diagnostic), "expected a mapping");
  }

  #[test]
  fn non_mapping_stages_mean_no_stages() {
    let diagnostic = error(&format!("{PLUGINS}stages: [build]\n"));
    assert_eq!(
      diagnostic.message(),
      "No stages defined. A pipeline needs at least one stage."
    );
  }

  #[test]
  fn missing_plugins_are_reported_at_the_plugins_span() {
    let diagnostic = error(STAGES);
    assert_eq!(
      diagnostic.message(),
      "No plugins declared. Every step runs a middleware from a plugin, so at least one is required."
    );
    assert_eq!(label(&diagnostic), "define at least one plugin");
  }

  #[test]
  fn non_sequence_plugins_mean_no_plugins() {
    let diagnostic = error(&format!("plugins:\n  tp: file://x.wasm\n{STAGES}"));
    assert!(diagnostic.message().starts_with("No plugins declared."));
  }

  #[test]
  fn duplicate_plugin_names_are_rejected() {
    let yaml = format!("{PLUGINS}  - name: tp\n    url: file://other.wasm\n{STAGES}");
    let diagnostic = error(&yaml);
    assert_eq!(
      diagnostic.message(),
      "Duplicate plugin name 'tp'. Plugin names must be unique."
    );
    assert_eq!(label(&diagnostic), "duplicate plugin name");
  }

  #[test]
  fn undeclared_plugin_is_underlined_at_the_run_entry() {
    let yaml = format!("{PLUGINS}stages:\n  build:\n    - run: other.go\n");
    let diagnostic = error(&yaml);
    assert_eq!(diagnostic.message(), "No plugin is declared with the alias 'other'.");
    assert_eq!(underlined(&yaml, &diagnostic), "other.go");
    assert_eq!(label(&diagnostic), "unknown plugin");
  }

  #[test]
  fn duplicate_keys_are_rejected_at_every_level() {
    let cases = [
      with_plugins_and_stages("name: a\nname: b\n"),
      plugin_yaml("    url: file://again.wasm\n"),
      plugin_yaml("    permissions:\n      exec: [a]\n      exec: [b]\n"),
      step_yaml("      run: tp.again\n"),
    ];
    for yaml in &cases {
      let diagnostic = error(yaml);
      assert!(diagnostic.message().starts_with("Duplicate key '"));
      assert_eq!(label(&diagnostic), "duplicate key");
    }
  }

  #[test]
  fn duplicate_key_underlines_the_second_occurrence() {
    let yaml = with_plugins_and_stages("name: a\nname: b\n");
    let diagnostic = error(&yaml);
    assert_eq!(diagnostic.message(), "Duplicate key 'name'.");
    let span = diagnostic.span().expect("span");
    assert_eq!(span.offset(), "name: a\n".len());
  }

  #[test]
  fn unknown_keys_are_rejected_at_every_level() {
    let cases = [
      (with_plugins_and_stages("nmae: x\n"), "Unknown configuration key 'nmae'."),
      (plugin_yaml("    urls: x\n"), "Unknown plugin key 'urls'."),
      (
        plugin_yaml("    permissions:\n      disk: x\n"),
        "Unknown permissions key 'disk'.",
      ),
      (step_yaml("      conditon: x\n"), "Unknown step key 'conditon'."),
    ];
    for (yaml, message) in &cases {
      let diagnostic = error(yaml);
      assert_eq!(diagnostic.message(), *message);
      assert_eq!(label(&diagnostic), "unknown key");
    }
  }

  #[test]
  fn non_scalar_keys_are_unknown_keys() {
    let cases = [
      with_plugins_and_stages("? [a]\n: b\n"),
      plugin_yaml("    ? [a]\n    : b\n"),
      plugin_yaml("    permissions:\n      ? [a]\n      : b\n"),
      step_yaml("      ? [a]\n      : b\n"),
    ];
    for yaml in &cases {
      let diagnostic = error(yaml);
      assert!(diagnostic.message().ends_with("key ''."), "{}", diagnostic.message());
    }
  }

  #[test]
  fn null_values_are_rejected_where_a_value_is_required() {
    let cases = [
      (
        with_plugins_and_stages("arguments:\n"),
        "Key 'arguments' expects a mapping, but has no value.",
      ),
      (
        with_plugins_and_stages("variables: ~\n"),
        "Key 'variables' expects a mapping, but has no value.",
      ),
      (
        format!("plugins:\n{STAGES}"),
        "Key 'plugins' expects a sequence of plugins, but has no value.",
      ),
      (
        format!("{PLUGINS}stages:\n"),
        "Key 'stages' expects a mapping of stages, but has no value.",
      ),
      (
        "plugins:\n  - name: tp\n    url:\nstages:\n  b:\n    - run: tp.go\n".to_string(),
        "Key 'url' expects a plugin URL, but has no value.",
      ),
      (
        format!("{PLUGINS}stages:\n  build:\n"),
        "Key 'build' expects a sequence of steps, but has no value.",
      ),
      (
        step_yaml("      condition:\n"),
        "Key 'condition' expects an expression, but has no value.",
      ),
      (
        step_yaml("      haltIf: ~\n"),
        "Key 'haltIf' expects an expression, but has no value.",
      ),
    ];
    for (yaml, message) in &cases {
      let diagnostic = error(yaml);
      assert_eq!(diagnostic.message(), *message);
      assert_eq!(label(&diagnostic), "missing value");
    }
  }

  #[test]
  fn non_scalar_string_map_values_are_rejected() {
    let yaml = with_plugins_and_stages("variables:\n  region: [eu, us]\n");
    let diagnostic = error(&yaml);
    assert_eq!(diagnostic.message(), "Expected a string value in variables.");
    assert_eq!(underlined(&yaml, &diagnostic), "[eu, us]");
    assert_eq!(label(&diagnostic), "expected a string");
  }

  #[test]
  fn plugin_must_be_a_mapping() {
    let diagnostic = error(&format!("plugins:\n  - just-a-name\n{STAGES}"));
    assert_eq!(diagnostic.message(), "Expected a mapping for a plugin.");
  }

  #[test]
  fn plugin_without_url_is_rejected() {
    let diagnostic = error(&format!("plugins:\n  - name: tp\n{STAGES}"));
    assert_eq!(diagnostic.message(), "Plugin 'tp' is missing a 'url' entry.");
    assert_eq!(label(&diagnostic), "add a 'url' entry");
  }

  #[test]
  fn unsupported_url_scheme_is_rejected() {
    let yaml = format!("plugins:\n  - name: tp\n    url: ftp://host/x.wasm\n{STAGES}");
    let diagnostic = error(&yaml);
    assert!(diagnostic.message().starts_with("Invalid plugin url: ftp://host/x.wasm."));
    assert_eq!(underlined(&yaml, &diagnostic), "ftp://host/x.wasm");
    assert_eq!(label(&diagnostic), "unsupported or malformed url");
  }

  #[test]
  fn url_without_scheme_is_rejected() {
    let diagnostic = error(&format!("plugins:\n  - name: tp\n    url: tp.wasm\n{STAGES}"));
    assert!(diagnostic.message().starts_with("Invalid plugin url: tp.wasm."));
  }

  #[test]
  fn non_scalar_url_is_rejected() {
    let diagnostic = error(&format!("plugins:\n  - name: tp\n    url: [a]\n{STAGES}"));
    assert!(diagnostic.message().starts_with("Invalid plugin url: ."));
  }

  #[test]
  fn permissions_must_be_a_mapping() {
    let diagnostic = error(&plugin_yaml("    permissions: [network]\n"));
    assert_eq!(diagnostic.message(), "Expected a mapping for a plugin's permissions.");
  }

  #[test]
  fn invalid_filesystem_level_is_rejected() {
    let yaml = plugin_yaml("    permissions:\n      filesystem: everything\n");
    let diagnostic = error(&yaml);
    assert_eq!(
      diagnostic.message(),
      "Invalid filesystem access: everything. Expected one of: none, read-only, read-write."
    );
    assert_eq!(underlined(&yaml, &diagnostic), "everything");
    assert_eq!(label(&diagnostic), "unrecognized filesystem access level");
  }

  #[test]
  fn steps_must_be_a_sequence() {
    let diagnostic = error(&format!("{PLUGINS}stages:\n  build:\n    run: tp.go\n"));
    assert_eq!(diagnostic.message(), "Expected a sequence for a stage's steps.");
    assert_eq!(label(&diagnostic), "expected a sequence");
  }

  #[test]
  fn step_must_be_a_mapping() {
    let diagnostic = error(&format!("{PLUGINS}stages:\n  build:\n    - tp.go\n"));
    assert_eq!(diagnostic.message(), "Expected a mapping for a step.");
  }

  #[test]
  fn step_without_run_is_rejected() {
    let diagnostic = error(&format!("{PLUGINS}stages:\n  build:\n    - name: lonely\n"));
    assert_eq!(diagnostic.message(), "Step 'lonely' is missing a 'run' entry.");
    assert_eq!(label(&diagnostic), "add a 'run: plugin.middleware' entry");
  }

  #[test]
  fn malformed_run_references_are_rejected() {
    for run in ["tp", ".go", "tp.", "''"] {
      let yaml = format!("{PLUGINS}stages:\n  build:\n    - run: {run}\n");
      let diagnostic = error(&yaml);
      assert!(
        diagnostic
          .message()
          .ends_with("is not a valid run reference; use the format 'plugin.middleware'.")
      );
      assert_eq!(label(&diagnostic), "expected 'plugin.middleware'");
    }
  }

  #[test]
  fn non_boolean_continue_on_error_is_rejected() {
    let yaml = step_yaml("      continueOnError: maybe\n");
    let diagnostic = error(&yaml);
    assert_eq!(
      diagnostic.message(),
      "Invalid continueOnError value: maybe. Expected 'true' or 'false'."
    );
    assert_eq!(underlined(&yaml, &diagnostic), "maybe");
    assert_eq!(label(&diagnostic), "expected 'true' or 'false'");
  }

  #[test]
  fn yaml_syntax_errors_point_at_the_problem() {
    let yaml = "name: [unclosed\n";
    let diagnostic = error(yaml);
    assert!(diagnostic.message().starts_with("Invalid YAML: "), "{}", diagnostic.message());
    assert_eq!(label(&diagnostic), "here");
    assert_eq!(diagnostic.span().expect("span").len(), 0);
  }

  #[test]
  fn undefined_alias_is_rejected() {
    let diagnostic = error(&with_plugins_and_stages("variables: *missing\n"));
    let message = diagnostic.message();
    assert!(
      message == "Unknown YAML alias: no matching anchor was defined." || message.starts_with("Invalid YAML: "),
      "{message}"
    );
  }

  #[test]
  fn spans_are_byte_offsets_even_after_multibyte_text() {
    let yaml = with_plugins_and_stages("name: café ☕\nnmae: x\n");
    let diagnostic = error(&yaml);
    assert_eq!(underlined(&yaml, &diagnostic), "nmae");
  }

  #[test]
  fn diagnostics_name_the_source_file() {
    let diagnostic = parse_config("- a\n", "release.yaml").expect_err("invalid");
    let source = diagnostic.source_code().expect("source code");
    let contents = source.read_span(&(0, 1).into(), 0, 0).expect("readable source");
    assert_eq!(contents.name(), Some("release.yaml"));
  }
}
