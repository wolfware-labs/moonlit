use std::sync::LazyLock;

use crate::expr::Resolve;
use crate::expr::value::Value;
use regex::{Captures, Regex};

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\(([^)]+)\)").unwrap());

pub fn substitute(input: &str, resolver: &dyn Resolve) -> Value {
    if input.trim().is_empty() {
        return Value::Null;
    }
    if let Some(caps) = PLACEHOLDER.captures(input) {
        let whole = caps.get(0).unwrap();
        if whole.start() == 0 && whole.end() == input.len() {
            let inner = caps.get(1).unwrap().as_str();
            return resolve_inner(inner, resolver).unwrap_or(Value::Null);
        }
    } else {
        return Value::Str(input.to_string());
    }
    Value::Str(substitute_with(input, resolver, |v| match v {
        Some(val) => val.to_display_string(),
        None => String::new(),
    }))
}

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
