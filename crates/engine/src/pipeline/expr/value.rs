use indexmap::IndexMap;
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
  Null,
  Str(String),
  List(Vec<Value>),
  Map(IndexMap<String, Value>),
}

impl Value {
  #[must_use]
  pub fn to_display_string(&self) -> String {
    match self {
      Value::Null => String::new(),
      Value::Str(s) => s.clone(),
      _ => self.to_json_string(),
    }
  }

  #[must_use]
  pub fn to_json(&self) -> serde_json::Value {
    match self {
      Value::Null => serde_json::Value::Null,
      Value::Str(s) => serde_json::Value::String(s.clone()),
      Value::List(items) => serde_json::Value::Array(items.iter().map(Value::to_json).collect()),
      Value::Map(m) => serde_json::Value::Object(m.iter().map(|(k, v)| (k.clone(), v.to_json())).collect()),
    }
  }

  #[must_use]
  pub fn from_json(j: &serde_json::Value) -> Value {
    match j {
      serde_json::Value::Null => Value::Null,
      serde_json::Value::Bool(b) => Value::Str(b.to_string()),
      serde_json::Value::Number(n) => Value::Str(n.to_string()),
      serde_json::Value::String(s) => Value::Str(s.clone()),
      serde_json::Value::Array(a) => Value::List(a.iter().map(Value::from_json).collect()),
      serde_json::Value::Object(o) => Value::Map(o.iter().map(|(k, v)| (k.clone(), Value::from_json(v))).collect()),
    }
  }

  #[must_use]
  pub fn to_json_string(&self) -> String {
    match self {
      Value::Null => "null".to_string(),
      Value::Str(s) => json_escape(s),
      Value::List(items) => {
        let inner: Vec<String> = items.iter().map(Value::to_json_string).collect();
        format!("[{}]", inner.join(","))
      }
      Value::Map(m) => {
        let inner: Vec<String> = m
          .iter()
          .map(|(k, v)| format!("{}:{}", json_escape(k), v.to_json_string()))
          .collect();
        format!("{{{}}}", inner.join(","))
      }
    }
  }
}
#[must_use]
fn json_escape(s: &str) -> String {
  let mut out = String::with_capacity(s.len() + 2);
  out.push('"');
  for c in s.chars() {
    match c {
      '"' => out.push_str("\\\""),
      '\\' => out.push_str("\\\\"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      c if (c as u32) < 0x20 => {
        let _ = write!(out, "\\u{:04x}", c as u32);
      }
      c => out.push(c),
    }
  }
  out.push('"');
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample() -> Value {
    Value::Map(IndexMap::from([
      ("name".to_string(), Value::Str("demo".to_string())),
      (
        "items".to_string(),
        Value::List(vec![Value::Null, Value::Str("x".to_string())]),
      ),
    ]))
  }

  #[test]
  fn display_string_is_raw_for_scalars_and_json_for_containers() {
    assert_eq!(Value::Null.to_display_string(), "");
    assert_eq!(Value::Str("plain".to_string()).to_display_string(), "plain");
    assert_eq!(sample().to_display_string(), r#"{"name":"demo","items":[null,"x"]}"#);
  }

  #[test]
  fn json_string_escapes_special_characters() {
    let value = Value::Str("q\" b\\ n\n r\r t\t c\u{1}".to_string());
    assert_eq!(value.to_json_string(), r#""q\" b\\ n\n r\r t\t c\u0001""#);
  }

  #[test]
  fn json_string_escapes_map_keys() {
    let value = Value::Map(IndexMap::from([("k\"ey".to_string(), Value::Null)]));
    assert_eq!(value.to_json_string(), r#"{"k\"ey":null}"#);
  }

  #[test]
  fn to_json_builds_the_matching_serde_value() {
    assert_eq!(
      sample().to_json(),
      serde_json::json!({ "name": "demo", "items": [null, "x"] })
    );
  }

  #[test]
  fn from_json_stringifies_booleans_and_numbers() {
    let json = serde_json::json!({ "flag": true, "count": 3, "ratio": 1.5, "list": [null, "s"] });
    let value = Value::from_json(&json);
    assert_eq!(
      value,
      Value::Map(IndexMap::from([
        ("flag".to_string(), Value::Str("true".to_string())),
        ("count".to_string(), Value::Str("3".to_string())),
        ("ratio".to_string(), Value::Str("1.5".to_string())),
        (
          "list".to_string(),
          Value::List(vec![Value::Null, Value::Str("s".to_string())]),
        ),
      ]))
    );
  }

  #[test]
  fn json_round_trip_preserves_strings_and_structure() {
    assert_eq!(Value::from_json(&sample().to_json()), sample());
  }
}
