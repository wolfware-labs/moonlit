//! Ergonomic result builder. Authors return a `MiddlewareResult<Output>`; the
//! `moonlit_plugin!` glue converts it to the WIT `middleware-result` via
//! `into_wit`, serializing the typed output into the ABI's `(key, json-text)`
//! output map. A serialization failure degrades the whole result to a loud
//! `failure(...)`.

/// A middleware outcome in ergonomic form, carrying a typed `Output` on success.
pub struct MiddlewareResult<T> {
    successful: bool,
    error_message: Option<String>,
    warnings: Vec<String>,
    output: Option<T>,
}

impl<T: serde::Serialize> MiddlewareResult<T> {
    /// A successful outcome carrying the middleware's typed output. Use
    /// `NoOutput` for middlewares that publish nothing.
    pub fn ok(output: T) -> Self {
        Self {
            successful: true,
            error_message: None,
            warnings: Vec::new(),
            output: Some(output),
        }
    }
}

impl<T> MiddlewareResult<T> {
    /// A failed outcome; carries no output.
    pub fn failure(msg: impl Into<String>) -> Self {
        Self {
            successful: false,
            error_message: Some(msg.into()),
            warnings: Vec::new(),
            output: None,
        }
    }

    #[must_use]
    pub fn with_warning(mut self, msg: impl Into<String>) -> Self {
        self.warnings.push(msg.into());
        self
    }

    /// Whether the middleware succeeded. Useful in `sdk::testing` unit tests.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.successful
    }

    /// The failure message, if this result is a failure.
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Warnings attached to this result.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

impl<T: serde::Serialize> MiddlewareResult<T> {
    /// Convert to the WIT record. The typed output is serialized to a JSON
    /// object and spread into the `(key, json-text)` output list (the field
    /// names become the map keys, so `steps.NAME.outputs.<field>` resolves). A
    /// non-object output or a serialization error degrades to a loud failure.
    pub fn into_wit(self) -> crate::bindings::MiddlewareResult {
        let mut output: Vec<(String, String)> = Vec::new();
        if let Some(value) = self.output {
            match serde_json::to_value(&value) {
                Ok(serde_json::Value::Object(map)) => {
                    for (k, v) in map {
                        output.push((k, v.to_string()));
                    }
                }
                // `NoOutput`-like empties (`{}`) land here as an object with no
                // entries; `()` serializes to Null. Both mean "no outputs".
                Ok(serde_json::Value::Null) => {}
                Ok(other) => {
                    return crate::bindings::MiddlewareResult {
                        successful: false,
                        error_message: Some(format!(
                            "middleware output must serialize to a JSON object, got {other}"
                        )),
                        warnings: self.warnings,
                        output: Vec::new(),
                    };
                }
                Err(e) => {
                    return crate::bindings::MiddlewareResult {
                        successful: false,
                        error_message: Some(format!("output serialization failed: {e}")),
                        warnings: self.warnings,
                        output: Vec::new(),
                    };
                }
            }
        }
        crate::bindings::MiddlewareResult {
            successful: self.successful,
            error_message: self.error_message,
            warnings: self.warnings,
            output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_is_successful_no_warnings() {
        let w = MiddlewareResult::ok(()).into_wit();
        assert!(w.successful);
        assert!(w.error_message.is_none());
        assert!(w.warnings.is_empty());
        assert!(w.output.is_empty());
    }

    #[test]
    fn no_output_serializes_to_empty() {
        let w = MiddlewareResult::ok(crate::NoOutput {}).into_wit();
        assert!(w.successful);
        assert!(w.output.is_empty());
    }

    #[test]
    fn failure_carries_message_and_is_not_successful() {
        let w = MiddlewareResult::<()>::failure("boom").into_wit();
        assert!(!w.successful);
        assert_eq!(w.error_message.as_deref(), Some("boom"));
    }

    #[test]
    fn with_warning_is_chainable() {
        let w = MiddlewareResult::ok(()).with_warning("careful").into_wit();
        assert!(w.successful);
        assert_eq!(w.warnings, vec!["careful".to_string()]);
    }

    #[test]
    fn typed_output_spreads_fields_into_json_values() {
        #[derive(serde::Serialize)]
        struct Out {
            tag: String,
            count: u32,
        }
        let w = MiddlewareResult::ok(Out {
            tag: "v1.2.3".to_string(),
            count: 7,
        })
        .into_wit();
        assert!(w.successful);
        // json-value is JSON text; strings are quoted, numbers bare.
        let map: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(map["tag"], "\"v1.2.3\"");
        assert_eq!(map["count"], "7");
    }

    #[test]
    fn accessors_expose_outcome() {
        let ok = MiddlewareResult::ok(()).with_warning("w");
        assert!(ok.is_success());
        assert_eq!(ok.error_message(), None);
        assert_eq!(ok.warnings(), &["w".to_string()]);

        let bad = MiddlewareResult::<()>::failure("boom");
        assert!(!bad.is_success());
        assert_eq!(bad.error_message(), Some("boom"));
        assert!(bad.warnings().is_empty());
    }

    #[test]
    fn output_serialize_error_degrades_to_failure() {
        // A field whose value can't be JSON-encoded (non-string map keys) makes
        // `serde_json::to_value` error, which `into_wit` surfaces as a failure.
        #[derive(serde::Serialize)]
        struct Bad {
            bad: std::collections::HashMap<(i32, i32), i32>,
        }
        let mut bad = std::collections::HashMap::new();
        bad.insert((1, 2), 3);
        let w = MiddlewareResult::ok(Bad { bad }).into_wit();
        assert!(!w.successful);
        assert!(
            w.error_message
                .as_deref()
                .unwrap()
                .contains("serialization"),
            "message should indicate a serialization failure; got {:?}",
            w.error_message
        );
    }
}
