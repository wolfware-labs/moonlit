use std::sync::LazyLock;

use crate::pipeline::expr::Resolve;
use crate::pipeline::expr::value::Value;
use regex::{Captures, Regex};

static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\(([^)]+)\)").unwrap());

#[must_use]
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

#[must_use]
pub(crate) fn substitute_with(input: &str, resolver: &dyn Resolve, render: impl Fn(Option<Value>) -> String) -> String {
  PLACEHOLDER
    .replace_all(input, |caps: &Captures| {
      render(resolve_inner(caps.get(1).unwrap().as_str(), resolver))
    })
    .into_owned()
}

#[must_use]
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
  use indexmap::IndexMap;
  use std::collections::HashMap;

  struct Table(HashMap<&'static str, Value>);

  impl Resolve for Table {
    fn resolve(&self, path: &str) -> Option<Value> {
      self.0.get(path).cloned()
    }
  }

  fn table() -> Table {
    Table(HashMap::from([
      ("args:version", Value::Str("1.2".to_string())),
      ("vars:region", Value::Str("eu".to_string())),
      (
        "output:s1",
        Value::Map(IndexMap::from([("k".to_string(), Value::Str("v".to_string()))])),
      ),
    ]))
  }

  #[test]
  fn blank_input_is_null() {
    assert_eq!(substitute("   ", &table()), Value::Null);
  }

  #[test]
  fn text_without_placeholders_is_kept() {
    assert_eq!(substitute("plain", &table()), Value::Str("plain".to_string()));
  }

  #[test]
  fn a_lone_placeholder_keeps_the_resolved_value_type() {
    assert_eq!(
      substitute("$(output:s1)", &table()),
      Value::Map(IndexMap::from([("k".to_string(), Value::Str("v".to_string()))]))
    );
  }

  #[test]
  fn a_lone_unresolved_placeholder_is_null() {
    assert_eq!(substitute("$(missing)", &table()), Value::Null);
  }

  #[test]
  fn placeholders_inside_text_render_as_strings() {
    assert_eq!(
      substitute("v$(args:version)-$(vars:region)-$(missing)!", &table()),
      Value::Str("v1.2-eu-!".to_string())
    );
  }

  #[test]
  fn containers_render_as_json_inside_text() {
    assert_eq!(
      substitute("data=$(output:s1)", &table()),
      Value::Str(r#"data={"k":"v"}"#.to_string())
    );
  }

  #[test]
  fn the_last_colon_separates_a_default_value() {
    assert_eq!(
      resolve_inner("args:missing:fallback", &table()),
      Some(Value::Str("fallback".to_string()))
    );
    assert_eq!(
      resolve_inner("args:version:fallback", &table()),
      Some(Value::Str("1.2".to_string()))
    );
  }

  #[test]
  fn full_paths_resolve_before_defaults_are_considered() {
    assert_eq!(resolve_inner(" vars:region ", &table()), Some(Value::Str("eu".to_string())));
  }

  #[test]
  fn blank_or_unknown_names_without_colons_resolve_to_nothing() {
    assert_eq!(resolve_inner("  ", &table()), None);
    assert_eq!(resolve_inner("unknown", &table()), None);
  }

  #[test]
  fn substitute_with_uses_the_given_renderer() {
    let rendered = substitute_with("[$(args:version)][$(nope)]", &table(), |value| match value {
      Some(v) => format!("<{}>", v.to_display_string()),
      None => "<none>".to_string(),
    });
    assert_eq!(rendered, "[<1.2>][<none>]");
  }
}
