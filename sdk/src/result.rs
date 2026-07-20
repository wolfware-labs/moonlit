//! Ergonomic result + output builders. Authors return a `MiddlewareResult`;
//! the `moonlit_plugin!` glue converts it to the WIT `middleware-result` via
//! `into_wit`. Output values serialize to JSON text (the ABI `json-value`);
//! a serialization failure degrades the whole result to a loud `failure(...)`.

/// A middleware outcome in ergonomic form.
pub struct MiddlewareResult {
    successful: bool,
    error_message: Option<String>,
    warnings: Vec<String>,
    /// (key, JSON-text value) — or an Err captured from a failed serialize.
    output: Vec<(String, Result<String, String>)>,
}

/// Output accumulator handed to `success_with`.
pub struct Output {
    entries: Vec<(String, Result<String, String>)>,
}

impl Output {
    /// Serialize `value` to a json-value. A failure is captured and later
    /// degrades the result to `failure` (keeps `execute -> MiddlewareResult`).
    pub fn set<T: serde::Serialize>(&mut self, key: impl Into<String>, value: T) {
        let key = key.into();
        let encoded = serde_json::to_string(&value).map_err(|e| e.to_string());
        self.entries.push((key, encoded));
    }
}

impl MiddlewareResult {
    pub fn success() -> Self {
        Self {
            successful: true,
            error_message: None,
            warnings: Vec::new(),
            output: Vec::new(),
        }
    }

    pub fn success_with(f: impl FnOnce(&mut Output)) -> Self {
        let mut out = Output {
            entries: Vec::new(),
        };
        f(&mut out);
        Self {
            successful: true,
            error_message: None,
            warnings: Vec::new(),
            output: out.entries,
        }
    }

    pub fn failure(msg: impl Into<String>) -> Self {
        Self {
            successful: false,
            error_message: Some(msg.into()),
            warnings: Vec::new(),
            output: Vec::new(),
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

    /// Convert to the WIT record. Any captured output-serialization error turns
    /// the whole result into a failure naming the offending key.
    pub fn into_wit(self) -> crate::bindings::MiddlewareResult {
        let mut output = Vec::with_capacity(self.output.len());
        for (key, encoded) in self.output {
            match encoded {
                Ok(json) => output.push((key, json)),
                Err(e) => {
                    return crate::bindings::MiddlewareResult {
                        successful: false,
                        error_message: Some(format!(
                            "output serialization failed for key '{key}': {e}"
                        )),
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
    fn success_is_successful_no_warnings() {
        let w = MiddlewareResult::success().into_wit();
        assert!(w.successful);
        assert!(w.error_message.is_none());
        assert!(w.warnings.is_empty());
        assert!(w.output.is_empty());
    }

    #[test]
    fn failure_carries_message_and_is_not_successful() {
        let w = MiddlewareResult::failure("boom").into_wit();
        assert!(!w.successful);
        assert_eq!(w.error_message.as_deref(), Some("boom"));
    }

    #[test]
    fn with_warning_is_chainable_on_success_and_failure() {
        let w = MiddlewareResult::success()
            .with_warning("careful")
            .into_wit();
        assert!(w.successful);
        assert_eq!(w.warnings, vec!["careful".to_string()]);
    }

    #[test]
    fn success_with_serializes_outputs_to_json_values() {
        let w = MiddlewareResult::success_with(|o| {
            o.set("tag", "v1.2.3");
            o.set("count", 7u32);
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
        let ok = MiddlewareResult::success_with(|o| o.set("k", 1)).with_warning("w");
        assert!(ok.is_success());
        assert_eq!(ok.error_message(), None);
        assert_eq!(ok.warnings(), &["w".to_string()]);

        let bad = MiddlewareResult::failure("boom");
        assert!(!bad.is_success());
        assert_eq!(bad.error_message(), Some("boom"));
        assert!(bad.warnings().is_empty());
    }

    #[test]
    fn output_serialize_error_degrades_to_failure() {
        // A map with non-string keys cannot be encoded as JSON -> serde_json errors,
        // which `into_wit` must surface as a failure naming the offending key.
        use std::collections::HashMap;
        let mut bad_map: HashMap<(i32, i32), i32> = HashMap::new();
        bad_map.insert((1, 2), 3);
        let w = MiddlewareResult::success_with(|o| {
            o.set("bad", bad_map);
        })
        .into_wit();
        assert!(!w.successful);
        assert!(
            w.error_message.as_deref().unwrap().contains("bad"),
            "message names the offending key"
        );
    }
}
