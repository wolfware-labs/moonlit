use crate::logging::LogLevel;

wasmtime::component::bindgen!({
    path: "../pdk/wit",
    world: "plugin-host",
    imports: { default: async | trappable },
    exports: { default: async },
    with: {
        "wasi": wasmtime_wasi::p2::bindings,
        "moonlit:plugin/process.child": crate::host::ChildProc,
    },
});

impl From<moonlit::plugin::types::LogLevel> for LogLevel {
    fn from(value: moonlit::plugin::types::LogLevel) -> Self {
        match value {
            moonlit::plugin::types::LogLevel::Debug => LogLevel::Debug,
            moonlit::plugin::types::LogLevel::Error => LogLevel::Error,
            moonlit::plugin::types::LogLevel::Info => LogLevel::Info,
            moonlit::plugin::types::LogLevel::Warn => LogLevel::Warn,
            moonlit::plugin::types::LogLevel::Trace => LogLevel::Trace,
        }
    }
}

impl From<crate::host::ReleaseContext> for ReleaseContext {
    fn from(value: crate::host::ReleaseContext) -> Self {
        ReleaseContext {
            working_directory: value.working_directory.clone(),
            step_name: value.step_name.clone(),
        }
    }
}

impl From<PluginMetadata> for crate::plugin::PluginMetadata {
    fn from(value: PluginMetadata) -> Self {
        Self {
            name: value.name,
            version: value.version,
            description: value.description,
            icon: value.icon,
        }
    }
}

impl From<MiddlewareResult>
    for Result<crate::pipeline::MiddlewareResult, crate::host::error::HostError>
{
    fn from(value: MiddlewareResult) -> Self {
        let mut output = Vec::with_capacity(value.output.len());
        for (k, json) in value.output {
            let value = json_str_to_value(&json, &format!("output key '{k}'"))?;
            output.push((k, value));
        }
        Ok(crate::pipeline::MiddlewareResult {
            successful: value.successful,
            error_message: value.error_message,
            warnings: value.warnings,
            output,
        })
    }
}

impl From<MiddlewareInfo> for crate::plugin::MiddlewareInfo {
    fn from(value: MiddlewareInfo) -> Self {
        Self {
            name: value.name,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
        }
    }
}

fn json_str_to_value(
    s: &str,
    context: &str,
) -> Result<serde_json::Value, crate::host::error::HostError> {
    serde_json::from_str(s).map_err(|source| crate::host::error::HostError::BadJson {
        context: context.to_string(),
        source,
    })
}
