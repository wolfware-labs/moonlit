//! ABI value bridging + the host error type + clean public value types.
//! The ABI speaks JSON strings (`type json-value = string`); this module converts
//! between the engine's `expr::Value`, `serde_json::Value`, and the JSON text form.

use crate::expr::value::Value;

/// Errors from the host layer. `init`'s domain `err(string)` is NOT a `HostError`
/// (it is a plugin-load outcome carried as `Result<_, String>`); a trap during any
/// call is a `HostError::Trap`.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum HostError {
    #[error("failed to instantiate plugin component: {0}")]
    #[diagnostic(code(moonlit::host::instantiate))]
    Instantiate(String),
    #[error("failed to link host interface: {0}")]
    #[diagnostic(code(moonlit::host::link))]
    Link(String),
    #[error("plugin trapped during {op}: {message}")]
    #[diagnostic(code(moonlit::host::trap))]
    Trap { op: String, message: String },
    #[error("plugin returned malformed JSON for {context}: {source}")]
    #[diagnostic(
        code(moonlit::host::bad_json),
        help("The plugin returned invalid JSON; check the plugin's output serialization logic")
    )]
    BadJson {
        context: String,
        source: serde_json::Error,
    },
    #[error(transparent)]
    #[diagnostic(code(moonlit::host::io))]
    Io(#[from] std::io::Error),
}

/// Clean public value types (mirror the WIT records; decouple the engine API from
/// bindgen-generated types). Mapping from generated records lands in later tasks.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareInfo {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareResult {
    pub successful: bool,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
    /// step outputs: key -> parsed JSON value (from the guest's json-value strings).
    pub output: Vec<(String, serde_json::Value)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseContext {
    pub working_directory: String,
    pub step_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Total map from the engine's runtime `Value` to `serde_json::Value` (same JSON
/// shape; all scalars are strings on the `Value` side).
pub fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => serde_json::Value::Array(items.iter().map(value_to_json).collect()),
        Value::Map(m) => serde_json::Value::Object(
            m.iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect(),
        ),
    }
}

/// Parse a guest-supplied `json-value` string, attributing failures to `context`.
pub fn json_str_to_value(s: &str, context: &str) -> Result<serde_json::Value, HostError> {
    serde_json::from_str(s).map_err(|source| HostError::BadJson {
        context: context.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{HostError, json_str_to_value, value_to_json};
    use crate::expr::value::Value;
    use indexmap::IndexMap;

    #[test]
    fn value_to_json_maps_every_variant() {
        let mut m = IndexMap::new();
        m.insert("k".to_string(), Value::Str("v".to_string()));
        m.insert("n".to_string(), Value::Null);
        m.insert(
            "l".to_string(),
            Value::List(vec![Value::Str("a".to_string())]),
        );
        let v = Value::Map(m);
        let j = value_to_json(&v);
        assert_eq!(j["k"], serde_json::json!("v"));
        assert_eq!(j["n"], serde_json::Value::Null);
        assert_eq!(j["l"], serde_json::json!(["a"]));
    }

    #[test]
    fn json_str_to_value_parses_and_reports_bad_json() {
        let ok = json_str_to_value("{\"a\":1}", "test").unwrap();
        assert_eq!(ok["a"], serde_json::json!(1));
        let err = json_str_to_value("{not json", "cfg").unwrap_err();
        assert!(matches!(err, HostError::BadJson { .. }));
    }
}
