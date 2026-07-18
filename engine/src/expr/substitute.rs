//! `$(...)` value substitution (§5.1). Whole-string mode returns the resolved value structurally;
//! embedded mode splices string forms. The `:default` fallback resolves the full inner path first,
//! then the segment before the last `:`, and finally treats the trailing segment as a literal.

use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::expr::accumulator::Resolve;
use crate::expr::value::Value;

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\(([^)]+)\)").unwrap());

/// Substitute `$(...)` occurrences in `input` per §5.1.
pub fn substitute(input: &str, resolver: &dyn Resolve) -> Value {
    if input.trim().is_empty() {
        return Value::Null;
    }
    // Whole-string mode: the entire input is exactly one placeholder.
    if let Some(caps) = PLACEHOLDER.captures(input) {
        let whole = caps.get(0).unwrap();
        if whole.start() == 0 && whole.end() == input.len() {
            let inner = caps.get(1).unwrap().as_str();
            return resolve_inner(inner, resolver).unwrap_or(Value::Null);
        }
    } else {
        return Value::Str(input.to_string());
    }
    // Embedded mode: splice each match's string form.
    Value::Str(substitute_with(input, resolver, |v| match v {
        Some(val) => val.to_display_string(),
        None => String::new(),
    }))
}

/// Embedded-style replacement with a custom renderer for each placeholder's resolved value.
pub(crate) fn substitute_with(
    input: &str,
    resolver: &dyn Resolve,
    render: impl Fn(Option<Value>) -> String,
) -> String {
    PLACEHOLDER
        .replace_all(input, |caps: &Captures| {
            render(resolve_inner(caps.get(1).unwrap().as_str(), resolver))
        })
        .into_owned()
}

/// The `:default` fallback (§5.1): resolve the full inner path first; if that fails and `inner`
/// contains a `:`, resolve the segment before the LAST `:` and — if it resolves — return ITS value;
/// only when that left segment also fails to resolve is the trailing segment used as a literal
/// default. Returns `None` when nothing resolves and there is no `:` to split on.
pub(crate) fn resolve_inner(inner: &str, resolver: &dyn Resolve) -> Option<Value> {
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }
    if let Some(v) = resolver.resolve(inner) {
        return Some(v);
    }
    if let Some(idx) = inner.rfind(':') {
        let (left, right) = (&inner[..idx], &inner[idx + 1..]);
        if let Some(v) = resolver.resolve(left) {
            return Some(v);
        }
        return Some(Value::Str(right.to_string()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::accumulator::Accumulator;
    use crate::expr::value::Value;

    fn map(pairs: Vec<(&str, Value)>) -> Value {
        Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }

    fn acc() -> Accumulator {
        // output: { version: { nextVersion: "1.2.0" }, tag: { name: "v1.2.0" } }
        let mut a = Accumulator::new();
        a.push(map(vec![(
            "output",
            map(vec![
                ("version", map(vec![("nextVersion", s("1.2.0"))])),
                ("tag", map(vec![("name", s("v1.2.0"))])),
                (
                    "commits",
                    map(vec![("details", Value::List(vec![s("a"), s("b")]))]),
                ),
            ]),
        )]));
        a
    }

    #[test]
    fn empty_or_whitespace_is_null() {
        assert_eq!(substitute("", &acc()), Value::Null);
        assert_eq!(substitute("   ", &acc()), Value::Null);
    }

    #[test]
    fn no_placeholder_returns_unchanged() {
        assert_eq!(substitute("plain text", &acc()), s("plain text"));
    }

    #[test]
    fn whole_string_returns_structured_value() {
        let v = substitute("$(output:commits:details)", &acc());
        assert_eq!(v, Value::List(vec![s("a"), s("b")]));
    }

    #[test]
    fn whole_string_missing_is_null() {
        // No `:` to split on and nothing resolves -> Null.
        assert_eq!(substitute("$(missingkey)", &acc()), Value::Null);
    }

    #[test]
    fn embedded_splices_string_and_missing_is_empty() {
        assert_eq!(
            substitute("v$(output:version:nextVersion)", &acc()),
            s("v1.2.0")
        );
        assert_eq!(substitute("x$(missingkey)y", &acc()), s("xy"));
    }

    #[test]
    fn default_used_only_when_left_unresolved() {
        // full path unresolved, left `BUILD_CONFIGURATION` unresolved -> literal default
        assert_eq!(
            substitute("$(BUILD_CONFIGURATION:Release)", &acc()),
            s("Release")
        );
        // full path resolves -> `name` is NOT treated as a default
        assert_eq!(substitute("$(output:tag:name)", &acc()), s("v1.2.0"));
    }

    #[test]
    fn default_ignored_when_left_resolves() {
        // Full path unresolved (can't descend into the `nextVersion` scalar), but the left segment
        // `output:version:nextVersion` resolves -> its value is used and `fallback` is ignored.
        assert_eq!(
            substitute("$(output:version:nextVersion:fallback)", &acc()),
            s("1.2.0")
        );
    }

    #[test]
    fn absent_leaf_returns_resolved_parent_map() {
        // Documented corner: `output:tag:absent` is unresolved, but `output:tag` resolves to a map,
        // so rule 2 returns that map rather than treating `absent` as a default.
        assert_eq!(
            substitute("$(output:tag:absent)", &acc()),
            map(vec![("name", s("v1.2.0"))])
        );
    }
}
