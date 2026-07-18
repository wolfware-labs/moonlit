//! Spanned node tree → typed [`PipelineConfig`]. The C#-converter port: case-insensitive schema
//! keys, last-wins on duplicates, unknown keys ignored, YAML-null filtering, verbatim arbitrary
//! map keys. `run`-format and `url`-scheme validation happen here, where the raw value and span
//! are in hand.

use indexmap::IndexMap;

use crate::config::diagnostic::{ConfigDiagnostic, Source};
use crate::config::model::{
    ConfigMap, ConfigValue, FilesystemAccess, Permissions, PipelineConfig, Plugin, PluginUrl, Run,
    Span, Spanned, Stage, Step,
};
use crate::config::tree::{Node, NodeValue};

/// Convert a spanned node tree into the raw (pre-cleanup) pipeline model.
pub fn convert(root: Node, src: &Source) -> Result<PipelineConfig, ConfigDiagnostic> {
    match &root.value {
        NodeValue::Null => Ok(empty_config(root.span)),
        NodeValue::Map(entries) => convert_root(entries, root.span, src),
        _ => Err(src.expected_mapping("the release configuration", root.span)),
    }
}

fn empty_config(span: Span) -> PipelineConfig {
    PipelineConfig {
        name: String::new(),
        arguments: IndexMap::new(),
        variables: IndexMap::new(),
        plugins: Spanned::new(Vec::new(), span),
        stages: Spanned::new(Vec::new(), span),
    }
}

fn convert_root(
    entries: &[(Node, Node)],
    root_span: Span,
    src: &Source,
) -> Result<PipelineConfig, ConfigDiagnostic> {
    let mut name: Option<String> = None;
    let mut arguments: Option<IndexMap<String, String>> = None;
    let mut variables: Option<IndexMap<String, String>> = None;
    let mut plugins: Option<Spanned<Vec<Plugin>>> = None;
    let mut stages: Option<Spanned<Vec<Stage>>> = None;

    for (key, value) in entries {
        match schema_key(key).as_deref() {
            Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
            Some("arguments") => arguments = Some(string_map(value, "arguments", src)?),
            Some("variables") => variables = Some(string_map(value, "variables", src)?),
            Some("plugins") => plugins = Some(convert_plugins(value, src)?),
            Some("stages") => stages = Some(convert_stages(value, src)?),
            _ => {} // unknown key ignored
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

/// Lowercased key of a scalar key node (schema matching); `None` for non-scalar keys.
fn schema_key(key: &Node) -> Option<String> {
    match &key.value {
        NodeValue::Scalar(raw) => Some(raw.to_lowercase()),
        _ => None,
    }
}

/// Verbatim key of a scalar key node (arbitrary maps); `None` for non-scalar keys.
fn raw_key(key: &Node) -> Option<&str> {
    match &key.value {
        NodeValue::Scalar(raw) => Some(raw.as_str()),
        _ => None,
    }
}

fn scalar_string(node: &Node) -> Option<String> {
    match &node.value {
        NodeValue::Scalar(raw) => Some(raw.clone()),
        _ => None,
    }
}

/// `arguments`/`variables`: map<string,string>, verbatim keys, null values dropped, last-wins.
fn string_map(
    node: &Node,
    context: &str,
    src: &Source,
) -> Result<IndexMap<String, String>, ConfigDiagnostic> {
    let mut out = IndexMap::new();
    if let NodeValue::Map(entries) = &node.value {
        for (key, value) in entries {
            let Some(k) = raw_key(key) else { continue };
            match &value.value {
                NodeValue::Null => {} // filtered
                NodeValue::Scalar(raw) => {
                    out.insert(k.to_string(), raw.clone());
                }
                _ => return Err(src.expected_string(context, value.span)),
            }
        }
    }
    Ok(out)
}

fn convert_plugins(node: &Node, src: &Source) -> Result<Spanned<Vec<Plugin>>, ConfigDiagnostic> {
    let mut plugins = Vec::new();
    if let NodeValue::Seq(items) = &node.value {
        for item in items {
            plugins.push(convert_plugin(item, src)?);
        }
    }
    Ok(Spanned::new(plugins, node.span))
}

fn convert_plugin(node: &Node, src: &Source) -> Result<Plugin, ConfigDiagnostic> {
    let NodeValue::Map(entries) = &node.value else {
        return Err(src.expected_mapping("a plugin", node.span));
    };
    let mut name: Option<String> = None;
    let mut url: Option<Spanned<PluginUrl>> = None;
    let mut config: Option<ConfigMap> = None;
    let mut permissions: Option<Permissions> = None;

    for (key, value) in entries {
        match schema_key(key).as_deref() {
            Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
            Some("url") => url = Some(convert_url(value, src)?),
            Some("config") => config = Some(config_map(value)),
            Some("permissions") => permissions = Some(convert_permissions(value)),
            _ => {}
        }
    }

    let name = name.unwrap_or_default();
    let url = match url {
        Some(u) => u,
        None => return Err(src.missing_url(&name, node.span)),
    };
    Ok(Plugin {
        name,
        url,
        config: config.unwrap_or_default(),
        permissions,
    })
}

fn convert_url(node: &Node, src: &Source) -> Result<Spanned<PluginUrl>, ConfigDiagnostic> {
    let raw = scalar_string(node).ok_or_else(|| src.invalid_url("", node.span))?;
    let scheme = raw.split_once("://").map(|(s, _)| s.to_lowercase());
    let url = match scheme.as_deref() {
        Some("oci") => PluginUrl::Oci(raw.clone()),
        Some("file") => PluginUrl::File(raw.clone()),
        Some("http") => PluginUrl::Http(raw.clone()),
        Some("https") => PluginUrl::Https(raw.clone()),
        Some("nuget") => return Err(src.nuget_removed(node.span)),
        _ => return Err(src.invalid_url(&raw, node.span)),
    };
    Ok(Spanned::new(url, node.span))
}

/// A plugin/step `config:` block. Delegates to [`to_config_value`] and unwraps the map.
fn config_map(node: &Node) -> ConfigMap {
    match to_config_value(node).value {
        ConfigValue::Map(m) => m,
        _ => ConfigMap::new(),
    }
}

/// Recursively convert a node into a spanned [`ConfigValue`]. Scalars stay raw strings; nulls are
/// KEPT here (null filtering applies only to arguments/variables/stages, not `config:`).
fn to_config_value(node: &Node) -> Spanned<ConfigValue> {
    let value = match &node.value {
        NodeValue::Null => ConfigValue::Null,
        NodeValue::Scalar(raw) => ConfigValue::String(raw.clone()),
        NodeValue::Seq(items) => ConfigValue::List(items.iter().map(to_config_value).collect()),
        NodeValue::Map(entries) => {
            let mut m = ConfigMap::new();
            for (key, val) in entries {
                if let NodeValue::Scalar(raw) = &key.value {
                    m.insert(raw.clone(), to_config_value(val)); // verbatim key, last-wins
                }
            }
            ConfigValue::Map(m)
        }
    };
    Spanned::new(value, node.span)
}

fn convert_permissions(node: &Node) -> Permissions {
    let mut p = Permissions::full_trust();
    if let NodeValue::Map(entries) = &node.value {
        for (key, value) in entries {
            match schema_key(key).as_deref() {
                Some("network") => p.network = string_list(value),
                Some("exec") => p.exec = string_list(value),
                Some("env") => p.env = string_list(value),
                Some("filesystem") => {
                    if let Some(fs) = scalar_string(value).and_then(|s| parse_fs(&s)) {
                        p.filesystem = fs;
                    }
                }
                _ => {}
            }
        }
    }
    p
}

fn string_list(node: &Node) -> Vec<String> {
    match &node.value {
        NodeValue::Seq(items) => items.iter().filter_map(scalar_string).collect(),
        NodeValue::Scalar(raw) => vec![raw.clone()],
        _ => Vec::new(),
    }
}

fn parse_fs(raw: &str) -> Option<FilesystemAccess> {
    match raw.to_lowercase().as_str() {
        "none" => Some(FilesystemAccess::None),
        "read-only" | "readonly" => Some(FilesystemAccess::ReadOnly),
        "read-write" | "readwrite" => Some(FilesystemAccess::ReadWrite),
        _ => None,
    }
}

fn convert_stages(node: &Node, src: &Source) -> Result<Spanned<Vec<Stage>>, ConfigDiagnostic> {
    let mut stages = Vec::new();
    if let NodeValue::Map(entries) = &node.value {
        for (key, value) in entries {
            let Some(stage_name) = raw_key(key) else {
                continue;
            };
            if matches!(value.value, NodeValue::Null) {
                continue; // null stage filtered
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

fn convert_steps(node: &Node, src: &Source) -> Result<Vec<Step>, ConfigDiagnostic> {
    let mut steps = Vec::new();
    if let NodeValue::Seq(items) = &node.value {
        for item in items {
            steps.push(convert_step(item, src)?);
        }
    }
    Ok(steps)
}

fn convert_step(node: &Node, src: &Source) -> Result<Step, ConfigDiagnostic> {
    let NodeValue::Map(entries) = &node.value else {
        return Err(src.expected_mapping("a step", node.span));
    };
    let mut name: Option<String> = None;
    let mut run: Option<Spanned<Run>> = None;
    let mut condition: Option<String> = None;
    let mut halt_if: Option<String> = None;
    let mut continue_on_error: Option<bool> = None;
    let mut config: Option<ConfigMap> = None;

    for (key, value) in entries {
        match schema_key(key).as_deref() {
            Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
            Some("run") => run = Some(convert_run(value, src)?),
            Some("condition") => condition = scalar_string(value),
            Some("haltif") => halt_if = scalar_string(value),
            Some("continueonerror") => continue_on_error = Some(parse_bool(value, src)?),
            Some("config") => config = Some(config_map(value)),
            _ => {}
        }
    }

    let name = name.unwrap_or_default();
    let run = match run {
        Some(r) => r,
        None => return Err(src.missing_run(&name, node.span)),
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

fn convert_run(node: &Node, src: &Source) -> Result<Spanned<Run>, ConfigDiagnostic> {
    let raw = scalar_string(node).unwrap_or_default();
    match raw.split_once('.') {
        Some((plugin, middleware)) if !plugin.is_empty() && !middleware.is_empty() => {
            Ok(Spanned::new(
                Run {
                    plugin: plugin.to_string(),
                    middleware: middleware.to_string(),
                },
                node.span,
            ))
        }
        _ => Err(src.invalid_run(&raw, node.span)),
    }
}

fn parse_bool(node: &Node, src: &Source) -> Result<bool, ConfigDiagnostic> {
    let raw = scalar_string(node).unwrap_or_default();
    match raw.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(src.invalid_bool("continueOnError", &raw, node.span)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::diagnostic::Source;
    use crate::config::model::{ConfigValue, FilesystemAccess, PluginUrl};
    use crate::config::tree::build_tree;

    fn parse(yaml: &str) -> Result<PipelineConfig, ConfigDiagnostic> {
        let src = Source::new(yaml, "release.yml");
        let root = build_tree(&src)?;
        convert(root, &src)
    }

    fn ok(yaml: &str) -> PipelineConfig {
        parse(yaml).expect("valid config")
    }

    const FULL: &str = "\
name: demo
plugins:
  - name: git
    url: oci://reg.example.com/wolfware/git:2.0.0
stages:
  build:
    - name: tag
      run: git.tag
";

    #[test]
    fn parses_a_full_pipeline() {
        let c = ok(FULL);
        assert_eq!(c.name, "demo");
        assert_eq!(c.plugins.value.len(), 1);
        assert_eq!(c.plugins.value[0].name, "git");
        assert_eq!(c.stages.value.len(), 1);
        assert_eq!(c.stages.value[0].name, "build");
        let step = &c.stages.value[0].steps[0];
        assert_eq!(step.run.value.plugin, "git");
        assert_eq!(step.run.value.middleware, "tag");
    }

    #[test]
    fn schema_keys_are_case_insensitive() {
        let c = ok(
            "Name: demo\nPLUGINS:\n  - Name: git\n    URL: file:///p.wasm\nStages:\n  s:\n    - name: a\n      RUN: git.x\n",
        );
        assert_eq!(c.name, "demo");
        assert_eq!(c.plugins.value[0].name, "git");
        assert_eq!(c.stages.value[0].steps[0].run.value.middleware, "x");
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let c = ok(
            "name: demo\nnope: 1\nplugins:\n  - name: git\n    url: file:///p.wasm\n    bogus: x\nstages:\n  s:\n    - name: a\n      run: git.x\n      mystery: 9\n",
        );
        assert_eq!(c.name, "demo");
        assert_eq!(c.plugins.value[0].name, "git");
    }

    #[test]
    fn duplicate_schema_key_is_last_wins() {
        let c = ok(
            "name: first\nname: second\nplugins:\n  - name: git\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: git.x\n",
        );
        assert_eq!(c.name, "second");
    }

    #[test]
    fn null_values_filtered_in_arguments_but_config_keeps_null() {
        let c = ok(
            "arguments:\n  a: keep\n  b:\nplugins:\n  - name: git\n    url: file:///p.wasm\n    config:\n      k:\nstages:\n  s:\n    - name: a\n      run: git.x\n",
        );
        assert_eq!(c.arguments.len(), 1);
        assert_eq!(c.arguments.get("a"), Some(&"keep".to_string()));
        let cfg = &c.plugins.value[0].config;
        assert!(matches!(cfg.get("k").unwrap().value, ConfigValue::Null));
    }

    #[test]
    fn arbitrary_config_keys_preserved_verbatim() {
        let c = ok(
            "plugins:\n  - name: git\n    url: file:///p.wasm\n    config:\n      MixedCase: 1\nstages:\n  s:\n    - name: a\n      run: git.x\n",
        );
        assert!(c.plugins.value[0].config.contains_key("MixedCase"));
    }

    #[test]
    fn run_splits_on_first_dot() {
        let c = ok(
            "plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.a.b.c\n",
        );
        let run = &c.stages.value[0].steps[0].run.value;
        assert_eq!(run.plugin, "p");
        assert_eq!(run.middleware, "a.b.c");
    }

    #[test]
    fn bad_run_format_is_invalid_run() {
        let err = parse("plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: nodot\n").unwrap_err();
        assert_eq!(
            err.message(),
            "Invalid run format: nodot. Expected format: 'plugin.middleware'"
        );
    }

    #[test]
    fn url_schemes_classified_and_nuget_rejected() {
        let c = ok(
            "plugins:\n  - name: a\n    url: oci://r/x:1\n  - name: b\n    url: file:///p.wasm\n  - name: c\n    url: https://h/p.wasm\nstages:\n  s:\n    - name: s1\n      run: a.x\n",
        );
        assert!(matches!(c.plugins.value[0].url.value, PluginUrl::Oci(_)));
        assert!(matches!(c.plugins.value[1].url.value, PluginUrl::File(_)));
        assert!(matches!(c.plugins.value[2].url.value, PluginUrl::Https(_)));

        let err = parse("plugins:\n  - name: a\n    url: nuget://old/pkg\nstages:\n  s:\n    - name: s1\n      run: a.x\n").unwrap_err();
        assert!(err.message().contains("nuget://") && err.message().contains("oci://"));
    }

    #[test]
    fn continue_on_error_parses_and_rejects_non_bool() {
        let c = ok(
            "plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.x\n      continueOnError: TRUE\n",
        );
        assert!(c.stages.value[0].steps[0].continue_on_error);

        let err = parse("plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.x\n      continueOnError: maybe\n").unwrap_err();
        assert!(err.message().contains("Expected 'true' or 'false'"));
    }

    #[test]
    fn dollar_expressions_preserved_verbatim() {
        let c = ok(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n    config:\n      v: \"$(output:version:next)\"\nstages:\n  s:\n    - name: a\n      run: p.x\n      condition: $(args:skip) == false\n",
        );
        let v = c.plugins.value[0].config.get("v").unwrap();
        assert!(matches!(&v.value, ConfigValue::String(s) if s == "$(output:version:next)"));
        assert_eq!(
            c.stages.value[0].steps[0].condition.as_deref(),
            Some("$(args:skip) == false")
        );
    }

    #[test]
    fn omitted_permissions_is_none_but_present_is_parsed() {
        let c = ok(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n    permissions:\n      network: [api.github.com]\n      filesystem: read-only\nstages:\n  s:\n    - name: a\n      run: p.x\n",
        );
        let perms = c.plugins.value[0].permissions.as_ref().expect("present");
        assert_eq!(perms.network, vec!["api.github.com".to_string()]);
        assert_eq!(perms.filesystem, FilesystemAccess::ReadOnly);
    }
}
