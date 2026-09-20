use crate::pipeline::config::diagnostic::{ConfigDiagnostic, Source};
use crate::pipeline::config::model::{
    ConfigMap, ConfigValue, FilesystemAccess, Permissions, PipelineConfig, Plugin, PluginUrl, Run,
    Span, Spanned, Stage, Step,
};
use crate::pipeline::config::tree::{Node, NodeValue};
use indexmap::IndexMap;

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

fn schema_key(key: &Node) -> Option<&str> {
    match &key.value {
        NodeValue::Scalar(raw) => Some(raw.as_str()),
        _ => None,
    }
}

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

fn config_map(node: &Node) -> ConfigMap {
    match to_config_value(node).value {
        ConfigValue::Map(m) => m,
        _ => ConfigMap::new(),
    }
}

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

fn convert_permissions(node: &Node, src: &Source) -> Result<Permissions, ConfigDiagnostic> {
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
