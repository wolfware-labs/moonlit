//! Spanned node tree → typed [`PipelineConfig`]. Schema keys are matched exactly, duplicates and
//! unknown keys are rejected, and arbitrary map keys are preserved verbatim. `run`-format and
//! `url`-scheme validation happen here, where the raw value and span are in hand.

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

    let mut seen: Vec<&str> = Vec::new();
    for (key, value) in entries {
        if let Some(k) = schema_key(key) {
            if seen.contains(&k) {
                return Err(src.duplicate_key(k, key.span));
            }
            seen.push(k);
        }
        match schema_key(key) {
            Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
            Some("arguments") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("arguments", "a mapping", value.span));
                }
                arguments = Some(string_map(value, "arguments", src)?)
            }
            Some("variables") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("variables", "a mapping", value.span));
                }
                variables = Some(string_map(value, "variables", src)?)
            }
            Some("plugins") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("plugins", "a sequence of plugins", value.span));
                }
                plugins = Some(convert_plugins(value, src)?)
            }
            Some("stages") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("stages", "a mapping of stages", value.span));
                }
                stages = Some(convert_stages(value, src)?)
            }
            other => {
                return Err(src.unknown_key(other.unwrap_or_default(), "configuration", key.span));
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

/// Verbatim key of a scalar key node, for exact schema matching; `None` for non-scalar keys.
/// Schema keys are case-sensitive: `Plugins` is not `plugins`.
fn schema_key(key: &Node) -> Option<&str> {
    match &key.value {
        NodeValue::Scalar(raw) => Some(raw.as_str()),
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

    let mut seen: Vec<&str> = Vec::new();
    for (key, value) in entries {
        if let Some(k) = schema_key(key) {
            if seen.contains(&k) {
                return Err(src.duplicate_key(k, key.span));
            }
            seen.push(k);
        }
        match schema_key(key) {
            Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
            Some("url") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("url", "a plugin URL", value.span));
                }
                url = Some(convert_url(value, src)?)
            }
            Some("config") => config = Some(config_map(value)),
            Some("permissions") => permissions = Some(convert_permissions(value, src)?),
            other => {
                return Err(src.unknown_key(other.unwrap_or_default(), "plugin", key.span));
            }
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

fn convert_permissions(node: &Node, src: &Source) -> Result<Permissions, ConfigDiagnostic> {
    // A non-mapping used to fall through to deny-all. That is safe, but silent: a plugin then
    // failed at run time with a capability error far from the line that actually caused it.
    let NodeValue::Map(_) = &node.value else {
        return Err(src.expected_mapping("a plugin's permissions", node.span));
    };
    let mut p = Permissions::deny();
    if let NodeValue::Map(entries) = &node.value {
        let mut seen: Vec<&str> = Vec::new();
        for (key, value) in entries {
            if let Some(k) = schema_key(key) {
                if seen.contains(&k) {
                    return Err(src.duplicate_key(k, key.span));
                }
                seen.push(k);
            }
            match schema_key(key) {
                Some("network") => p.network = string_list(value),
                Some("exec") => p.exec = string_list(value),
                Some("env") => p.env = string_list(value),
                Some("filesystem") => {
                    if let Some(s) = scalar_string(value) {
                        p.filesystem =
                            parse_fs(&s).ok_or_else(|| src.invalid_filesystem(&s, value.span))?;
                    }
                }
                other => {
                    return Err(src.unknown_key(
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
                return Err(src.null_value(stage_name, "a sequence of steps", value.span));
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
    // Anything other than a sequence used to fall through to an empty Vec, so a stage with a
    // malformed body ran with no steps and reported success.
    let NodeValue::Seq(items) = &node.value else {
        return Err(src.expected_sequence("a stage's steps", node.span));
    };
    let mut steps = Vec::new();
    for item in items {
        steps.push(convert_step(item, src)?);
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

    let mut seen: Vec<&str> = Vec::new();
    for (key, value) in entries {
        if let Some(k) = schema_key(key) {
            if seen.contains(&k) {
                return Err(src.duplicate_key(k, key.span));
            }
            seen.push(k);
        }
        match schema_key(key) {
            Some("name") => name = Some(scalar_string(value).unwrap_or_default()),
            Some("run") => run = Some(convert_run(value, src)?),
            Some("condition") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("condition", "an expression", value.span));
                }
                condition = scalar_string(value)
            }
            Some("haltIf") => {
                if matches!(value.value, NodeValue::Null) {
                    return Err(src.null_value("haltIf", "an expression", value.span));
                }
                halt_if = scalar_string(value)
            }
            Some("continueOnError") => continue_on_error = Some(parse_bool(value, src)?),
            Some("config") => config = Some(config_map(value)),
            other => {
                return Err(src.unknown_key(other.unwrap_or_default(), "step", key.span));
            }
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
    fn canonical_schema_keys_parse() {
        let c = ok(
            "name: demo\nplugins:\n  - name: git\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: git.x\n",
        );
        assert_eq!(c.name, "demo");
        assert_eq!(c.plugins.value[0].name, "git");
        assert_eq!(c.stages.value[0].steps[0].run.value.middleware, "x");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let yaml = "name: x\nnotAKey: 1\n";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn duplicate_schema_key_is_rejected_not_last_wins() {
        // Previously last-wins silently discarded the first `name`. Duplicate schema keys
        // are now an error (see `duplicate_schema_keys_are_rejected` in diagnostic.rs tests).
        let err = parse(
            "name: first\nname: second\nplugins:\n  - name: git\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: git.x\n",
        )
        .unwrap_err();
        assert!(err.message().contains("name"), "got: {}", err.message());
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
            "'nodot' is not a valid run reference; use the format 'plugin.middleware'."
        );
    }

    #[test]
    fn url_schemes_classified() {
        let c = ok(
            "plugins:\n  - name: a\n    url: oci://r/x:1\n  - name: b\n    url: file:///p.wasm\n  - name: c\n    url: https://h/p.wasm\nstages:\n  s:\n    - name: s1\n      run: a.x\n",
        );
        assert!(matches!(c.plugins.value[0].url.value, PluginUrl::Oci(_)));
        assert!(matches!(c.plugins.value[1].url.value, PluginUrl::File(_)));
        assert!(matches!(c.plugins.value[2].url.value, PluginUrl::Https(_)));
    }

    #[test]
    fn nuget_scheme_is_a_generic_invalid_url() {
        let err = parse("plugins:\n  - name: a\n    url: nuget://old/pkg\nstages:\n  s:\n    - name: s1\n      run: a.x\n").unwrap_err();
        assert!(err.message().contains("Invalid plugin url"));
        assert!(!err.message().to_lowercase().contains("removed"));
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
        assert_eq!(perms.exec, Vec::<String>::new()); // deny base: unnamed key stays denied
        assert_eq!(perms.env, Vec::<String>::new()); // deny base: unnamed key stays denied
    }

    #[test]
    fn invalid_filesystem_value_errors_instead_of_failing_open() {
        let err = parse(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n    permissions:\n      filesystem: readonyl\nstages:\n  s:\n    - name: a\n      run: p.x\n",
        )
        .unwrap_err();
        assert_eq!(
            err.message(),
            "Invalid filesystem access: readonyl. Expected one of: none, read-only, read-write."
        );
    }

    #[test]
    fn missing_url_is_reported() {
        let err = parse("plugins:\n  - name: p\nstages:\n  s:\n    - name: a\n      run: p.x\n")
            .unwrap_err();
        assert_eq!(err.message(), "Plugin 'p' is missing a 'url' entry.");
    }

    #[test]
    fn missing_run_is_reported() {
        let err =
            parse("plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n")
                .unwrap_err();
        assert_eq!(err.message(), "Step 'a' is missing a 'run' entry.");
    }

    #[test]
    fn non_mapping_plugin_entry_is_expected_mapping() {
        let err = parse("plugins:\n  - not-a-map\nstages:\n  s:\n    - name: a\n      run: p.x\n")
            .unwrap_err();
        assert_eq!(err.message(), "Expected a mapping for a plugin.");
    }

    #[test]
    fn non_mapping_step_is_expected_mapping() {
        let err = parse(
            "plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - not-a-map\n",
        )
        .unwrap_err();
        assert_eq!(err.message(), "Expected a mapping for a step.");
    }

    #[test]
    fn non_scalar_argument_value_is_expected_string() {
        let err = parse(
            "arguments:\n  a: [1, 2]\nplugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.x\n",
        )
        .unwrap_err();
        assert_eq!(err.message(), "Expected a string value in arguments.");
    }

    #[test]
    fn non_scalar_variable_value_is_expected_string() {
        let err = parse(
            "variables:\n  v:\n    nested: true\nplugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.x\n",
        )
        .unwrap_err();
        assert_eq!(err.message(), "Expected a string value in variables.");
    }

    #[test]
    fn lowercased_key_no_longer_matches_the_schema() {
        // Before task 5 `haltif` matched `haltIf` because keys were lowercased. Case
        // sensitivity means it is simply not a schema key, and unknown schema keys are
        // now an error rather than being silently skipped.
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "stages:\n  s:\n    - name: a\n      run: p.x\n      haltif: 'true'\n",
        );
        let err = parse(yaml).unwrap_err();
        assert!(
            err.message().contains("haltif"),
            "`haltif` must be reported as an unknown key, got: {}",
            err.message()
        );
    }

    #[test]
    fn unknown_top_level_key_is_rejected() {
        // The typo this exists to catch.
        let yaml = "pluigns:\n  - name: p\n";
        let err = parse(yaml).unwrap_err();
        assert!(
            err.message().contains("pluigns"),
            "the offending key must be named, got: {}",
            err.message()
        );
        assert!(err.span().is_some(), "the diagnostic must point at the key");
    }

    #[test]
    fn a_capitalised_schema_key_is_rejected() {
        // Case sensitivity landed in task 5; with unknown keys now an error, a
        // case-mismatched key is reported rather than silently skipped.
        let yaml = "Plugins:\n  - name: p\n    url: file:///p.wasm\n";
        let err = parse(yaml).unwrap_err();
        assert!(
            err.message().contains("Plugins"),
            "capitalised schema key must be reported, got: {}",
            err.message()
        );
    }

    #[test]
    fn unknown_step_key_is_rejected() {
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "stages:\n  s:\n    - name: a\n      run: p.x\n      contineOnError: true\n",
        );
        let err = parse(yaml).unwrap_err();
        assert!(
            err.message().contains("contineOnError"),
            "got: {}",
            err.message()
        );
    }

    #[test]
    fn arbitrary_plugin_config_keys_are_still_free_form() {
        // Only SCHEMA keys are constrained. A plugin's own config is the plugin's contract.
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n    config:\n      anythingAtAll: 1\n",
            "stages:\n  s:\n    - name: a\n      run: p.x\n",
        );
        assert!(parse(yaml).is_ok());
    }

    #[test]
    fn duplicate_schema_keys_are_rejected() {
        // YAML forbids duplicate mapping keys; last-wins silently discarded the first.
        let yaml = "name: a\nname: b\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("name"), "got: {}", err.message());
        assert!(err.span().is_some());
    }

    #[test]
    fn null_under_a_schema_key_is_rejected() {
        let yaml = "plugins:\nstages:\n  s:\n    - name: a\n      run: p.x\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("plugins"), "got: {}", err.message());
    }

    #[test]
    fn null_inside_plugin_config_is_preserved() {
        // A plugin's config is the plugin's contract: null may be meaningful there.
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n    config:\n      maybe:\n",
            "stages:\n  s:\n    - name: a\n      run: p.x\n",
        );
        assert!(parse(yaml).is_ok());
    }

    #[test]
    fn canonical_camel_case_keys_parse() {
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "stages:\n  s:\n    - name: a\n      run: p.x\n",
            "      haltIf: 'true'\n      continueOnError: true\n",
        );
        let c = ok(yaml);
        let step = &c.stages.value[0].steps[0];
        assert_eq!(step.halt_if.as_deref(), Some("true"));
        assert!(step.continue_on_error);
    }

    #[test]
    fn unknown_plugin_key_is_rejected() {
        let yaml = "plugins:\n  - name: p\n    url: file:///p.wasm\n    bogusKey: 1\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("bogusKey"), "got: {}", err.message());
        assert!(err.span().is_some());
    }

    #[test]
    fn unknown_permissions_key_is_rejected() {
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "    permissions:\n      netwrok: []\n",
        );
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("netwrok"), "got: {}", err.message());
    }

    #[test]
    fn duplicate_key_in_a_plugin_is_rejected() {
        let yaml = "plugins:\n  - name: p\n    name: q\n    url: file:///p.wasm\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("name"), "got: {}", err.message());
    }

    #[test]
    fn null_url_is_rejected() {
        let yaml = "plugins:\n  - name: p\n    url:\n";
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("url"), "got: {}", err.message());
    }

    /// A stage body that is not a sequence used to yield an EMPTY stage that ran
    /// and reported success — the worst shape of silent acceptance in the parser.
    #[test]
    fn a_stage_whose_steps_are_not_a_sequence_is_rejected() {
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "stages:\n  build: oops\n",
        );
        let err = parse(yaml).unwrap_err();
        assert!(err.span().is_some(), "must point at the offending value");
        assert!(
            err.message().to_lowercase().contains("sequence"),
            "got: {}",
            err.message()
        );
    }

    #[test]
    fn a_null_stage_body_is_rejected() {
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "stages:\n  build:\n",
        );
        let err = parse(yaml).unwrap_err();
        assert!(err.message().contains("build"), "got: {}", err.message());
    }

    /// A malformed `permissions:` silently produced deny-all. Safe, but the plugin
    /// then failed at run time with a capability error far from the real cause.
    #[test]
    fn permissions_that_are_not_a_mapping_are_rejected() {
        let yaml = "plugins:\n  - name: p\n    url: file:///p.wasm\n    permissions: read-only\n";
        let err = parse(yaml).unwrap_err();
        assert!(
            err.message().to_lowercase().contains("mapping"),
            "got: {}",
            err.message()
        );
    }

    #[test]
    fn null_condition_and_halt_if_are_rejected() {
        let base = "plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.x\n";
        for key in ["condition", "haltIf"] {
            let yaml = format!("{base}      {key}:\n");
            let err = parse(&yaml).unwrap_err();
            assert!(err.message().contains(key), "{key}: got: {}", err.message());
        }
    }

    /// The guards above must not make valid pipelines stricter than intended.
    #[test]
    fn a_fully_populated_pipeline_still_parses() {
        let yaml = concat!(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "    permissions:\n      network: [\"example.com\"]\n",
            "stages:\n  s:\n    - name: a\n      run: p.x\n",
            "      condition: 'true'\n      haltIf: 'false'\n",
        );
        let c = ok(yaml);
        assert_eq!(c.stages.value[0].steps.len(), 1);
    }
}
