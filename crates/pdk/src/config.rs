//! Config binding with §5.4 string coercion. The engine sends config JSON in
//! which every scalar is a string (see engine `value_to_json`); typed fields
//! parse those strings in the fixed order bool -> i64 -> f64 -> datetime ->
//! String. `deserialize_any` (dynamic targets) applies the same order, with
//! datetime collapsing to a string (serde has no datetime type).

use serde::de::{self, Deserializer, IntoDeserializer, Visitor};
use serde::forward_to_deserialize_any;
use std::fmt;

/// A config binding failure (serde message, with a field path when serde
/// provides one).
#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "config error: {}", self.0)
  }
}
impl std::error::Error for ConfigError {}
impl de::Error for ConfigError {
  fn custom<T: fmt::Display>(msg: T) -> Self {
    ConfigError(msg.to_string())
  }
}

/// Deserialize `T` from a json-value string, the form the engine sends.
///
/// # Examples
///
/// ```
/// use moonlit_pdk::config::from_json_value;
///
/// #[derive(serde::Deserialize, Default, Debug, PartialEq)]
/// #[serde(default)]
/// struct Cfg { port: i64, debug: bool }
///
/// // Every scalar arrives as a string; typed fields coerce.
/// let cfg: Cfg = from_json_value(r#"{"port":"8080","debug":"true"}"#).unwrap();
/// assert_eq!(cfg, Cfg { port: 8080, debug: true });
/// ```
///
/// # Errors
///
/// Returns [`ConfigError`] if the text is not valid JSON, or if a field cannot
/// be coerced to its declared type.
pub fn from_json_value<T: de::DeserializeOwned>(json: &str) -> Result<T, ConfigError> {
  let value: serde_json::Value = serde_json::from_str(json).map_err(|e| ConfigError(e.to_string()))?;
  from_serde_value(value)
}

/// Deserialize `T` from an already-parsed `serde_json::Value`.
pub fn from_serde_value<T: de::DeserializeOwned>(value: serde_json::Value) -> Result<T, ConfigError> {
  T::deserialize(Coerce(value))
}

struct Coerce(serde_json::Value);

impl Coerce {
  fn parse_i64(&self) -> Result<i64, ConfigError> {
    match &self.0 {
      serde_json::Value::String(s) => s
        .trim()
        .parse::<i64>()
        .map_err(|_| ConfigError(format!("expected integer, got {s:?}"))),
      serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| ConfigError(format!("expected integer, got {n}"))),
      other => Err(ConfigError(format!("expected integer, got {other}"))),
    }
  }
  fn parse_u64(&self) -> Result<u64, ConfigError> {
    match &self.0 {
      serde_json::Value::String(s) => s
        .trim()
        .parse::<u64>()
        .map_err(|_| ConfigError(format!("expected unsigned integer, got {s:?}"))),
      serde_json::Value::Number(n) => n
        .as_u64()
        .ok_or_else(|| ConfigError(format!("expected unsigned integer, got {n}"))),
      other => Err(ConfigError(format!("expected unsigned integer, got {other}"))),
    }
  }
  fn parse_f64(&self) -> Result<f64, ConfigError> {
    match &self.0 {
      serde_json::Value::String(s) => s
        .trim()
        .parse::<f64>()
        .map_err(|_| ConfigError(format!("expected number, got {s:?}"))),
      serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| ConfigError(format!("expected number, got {n}"))),
      other => Err(ConfigError(format!("expected number, got {other}"))),
    }
  }
  fn parse_bool(&self) -> Result<bool, ConfigError> {
    match &self.0 {
      serde_json::Value::Bool(b) => Ok(*b),
      serde_json::Value::String(s) if s.eq_ignore_ascii_case("true") => Ok(true),
      serde_json::Value::String(s) if s.eq_ignore_ascii_case("false") => Ok(false),
      other => Err(ConfigError(format!("expected bool, got {other}"))),
    }
  }
}

impl<'de> Deserializer<'de> for Coerce {
  type Error = ConfigError;

  fn deserialize_bool<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_bool(self.parse_bool()?)
  }
  fn deserialize_i8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_i64(self.parse_i64()?)
  }
  fn deserialize_i16<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_i64(self.parse_i64()?)
  }
  fn deserialize_i32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_i64(self.parse_i64()?)
  }
  fn deserialize_i64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_i64(self.parse_i64()?)
  }
  fn deserialize_u8<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_u64(self.parse_u64()?)
  }
  fn deserialize_u16<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_u64(self.parse_u64()?)
  }
  fn deserialize_u32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_u64(self.parse_u64()?)
  }
  fn deserialize_u64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_u64(self.parse_u64()?)
  }
  fn deserialize_f32<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_f64(self.parse_f64()?)
  }
  fn deserialize_f64<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_f64(self.parse_f64()?)
  }

  fn deserialize_str<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    match self.0 {
      serde_json::Value::String(s) => v.visit_string(s),
      other => Err(ConfigError(format!("expected string, got {other}"))),
    }
  }
  fn deserialize_string<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    self.deserialize_str(v)
  }

  fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    match self.0 {
      serde_json::Value::Null => v.visit_none(),
      other => v.visit_some(Coerce(other)),
    }
  }
  fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    match self.0 {
      serde_json::Value::Null => v.visit_unit(),
      other => Err(ConfigError(format!("expected null, got {other}"))),
    }
  }
  fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, v: V) -> Result<V::Value, Self::Error> {
    self.deserialize_unit(v)
  }
  fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, v: V) -> Result<V::Value, Self::Error> {
    v.visit_newtype_struct(self)
  }

  fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    match self.0 {
      serde_json::Value::Array(items) => v.visit_seq(SeqAccess { iter: items.into_iter() }),
      other => Err(ConfigError(format!("expected array, got {other}"))),
    }
  }
  fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value, Self::Error> {
    self.deserialize_seq(v)
  }
  fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, v: V) -> Result<V::Value, Self::Error> {
    self.deserialize_seq(v)
  }

  fn deserialize_map<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    match self.0 {
      serde_json::Value::Object(map) => v.visit_map(MapAccess {
        iter: map.into_iter(),
        value: None,
      }),
      other => Err(ConfigError(format!("expected object, got {other}"))),
    }
  }
  fn deserialize_struct<V: Visitor<'de>>(
    self,
    _name: &'static str,
    _fields: &'static [&'static str],
    v: V,
  ) -> Result<V::Value, Self::Error> {
    self.deserialize_map(v)
  }

  fn deserialize_enum<V: Visitor<'de>>(
    self,
    _name: &'static str,
    _variants: &'static [&'static str],
    v: V,
  ) -> Result<V::Value, Self::Error> {
    match self.0 {
      // Unit variant encoded as a bare string.
      serde_json::Value::String(s) => v.visit_enum(s.into_deserializer()),
      // Newtype/struct/tuple variant encoded as a single-key object.
      serde_json::Value::Object(map) if map.len() == 1 => {
        let (tag, val) = map.into_iter().next().unwrap();
        v.visit_enum(EnumAccess { tag, value: val })
      }
      other => Err(ConfigError(format!("expected enum, got {other}"))),
    }
  }

  fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    v.visit_unit()
  }

  fn deserialize_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
    match self.0 {
      serde_json::Value::Null => v.visit_unit(),
      serde_json::Value::Bool(b) => v.visit_bool(b),
      serde_json::Value::Number(n) => {
        if let Some(i) = n.as_i64() {
          v.visit_i64(i)
        } else if let Some(u) = n.as_u64() {
          v.visit_u64(u)
        } else {
          v.visit_f64(n.as_f64().unwrap())
        }
      }
      serde_json::Value::String(s) => {
        // §5.4 order: bool -> i64 -> f64 -> (datetime as string) -> String.
        if s.eq_ignore_ascii_case("true") {
          return v.visit_bool(true);
        }
        if s.eq_ignore_ascii_case("false") {
          return v.visit_bool(false);
        }
        if let Ok(i) = s.parse::<i64>() {
          return v.visit_i64(i);
        }
        if let Ok(f) = s.parse::<f64>() {
          return v.visit_f64(f);
        }
        v.visit_string(s)
      }
      serde_json::Value::Array(items) => v.visit_seq(SeqAccess { iter: items.into_iter() }),
      serde_json::Value::Object(map) => v.visit_map(MapAccess {
        iter: map.into_iter(),
        value: None,
      }),
    }
  }

  forward_to_deserialize_any! { char bytes byte_buf identifier }
}

struct SeqAccess {
  iter: std::vec::IntoIter<serde_json::Value>,
}
impl<'de> de::SeqAccess<'de> for SeqAccess {
  type Error = ConfigError;
  fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
    match self.iter.next() {
      Some(v) => seed.deserialize(Coerce(v)).map(Some),
      None => Ok(None),
    }
  }
}

struct MapAccess {
  iter: serde_json::map::IntoIter,
  value: Option<serde_json::Value>,
}
impl<'de> de::MapAccess<'de> for MapAccess {
  type Error = ConfigError;
  fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
    match self.iter.next() {
      Some((k, v)) => {
        self.value = Some(v);
        // Keys are plain strings; feed them through a string deserializer.
        seed.deserialize(k.into_deserializer()).map(Some)
      }
      None => Ok(None),
    }
  }
  fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {
    let v = self.value.take().expect("next_value_seed after next_key_seed");
    seed.deserialize(Coerce(v))
  }
}

struct EnumAccess {
  tag: String,
  value: serde_json::Value,
}
impl<'de> de::EnumAccess<'de> for EnumAccess {
  type Error = ConfigError;
  type Variant = VariantAccess;
  fn variant_seed<V: de::DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> {
    let variant = seed.deserialize(self.tag.into_deserializer())?;
    Ok((variant, VariantAccess { value: self.value }))
  }
}

struct VariantAccess {
  value: serde_json::Value,
}
impl<'de> de::VariantAccess<'de> for VariantAccess {
  type Error = ConfigError;
  fn unit_variant(self) -> Result<(), Self::Error> {
    Ok(())
  }
  fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Self::Error> {
    seed.deserialize(Coerce(self.value))
  }
  fn tuple_variant<V: Visitor<'de>>(self, _len: usize, v: V) -> Result<V::Value, Self::Error> {
    Coerce(self.value).deserialize_seq(v)
  }
  fn struct_variant<V: Visitor<'de>>(self, _fields: &'static [&'static str], v: V) -> Result<V::Value, Self::Error> {
    Coerce(self.value).deserialize_map(v)
  }
}

// `de::IntoDeserializer` for the string values above uses serde's built-in
// `StrDeserializer<ConfigError>`; the `Error` type unifies via our `de::Error`.

#[cfg(test)]
mod tests {
  use super::*;
  use serde::Deserialize;

  #[derive(Deserialize, Default, PartialEq, Debug)]
  #[serde(rename_all = "camelCase", default)]
  struct Cfg {
    port: i64,
    enabled: bool,
    ratio: f64,
    label: String,
    retries: Option<u32>,
    tags: Vec<String>,
  }

  #[test]
  fn coerces_string_scalars_into_typed_fields() {
    // Engine sends every scalar as a JSON string.
    let json = r#"{"port":"5","enabled":"true","ratio":"1.5","label":"hi",
                       "retries":"3","tags":["a","b"]}"#;
    let cfg: Cfg = from_json_value(json).unwrap();
    assert_eq!(
      cfg,
      Cfg {
        port: 5,
        enabled: true,
        ratio: 1.5,
        label: "hi".into(),
        retries: Some(3),
        tags: vec!["a".into(), "b".into()],
      }
    );
  }

  #[test]
  fn case_insensitive_bool() {
    let json = r#"{"enabled":"FALSE"}"#;
    let cfg: Cfg = from_json_value(json).unwrap();
    assert!(!cfg.enabled);
  }

  #[test]
  fn null_becomes_none_and_defaults_apply() {
    let json = r#"{"retries":null}"#;
    let cfg: Cfg = from_json_value(json).unwrap();
    assert_eq!(cfg.retries, None);
    assert_eq!(cfg.port, 0); // serde default
  }

  #[test]
  fn native_json_scalars_also_accepted() {
    // Robustness: a bare JSON number/bool still binds.
    let json = r#"{"port":5,"enabled":true}"#;
    let cfg: Cfg = from_json_value(json).unwrap();
    assert_eq!(cfg.port, 5);
    assert!(cfg.enabled);
  }

  #[test]
  fn deserialize_any_applies_5_4_order() {
    // A dynamic target sees §5.4-typed scalars.
    let v: serde_json::Value = from_json_value(r#""42""#).unwrap();
    assert_eq!(v, serde_json::json!(42));
    let v: serde_json::Value = from_json_value(r#""true""#).unwrap();
    assert_eq!(v, serde_json::json!(true));
    let v: serde_json::Value = from_json_value(r#""3.5""#).unwrap();
    assert_eq!(v, serde_json::json!(3.5));
    let v: serde_json::Value = from_json_value(r#""hello""#).unwrap();
    assert_eq!(v, serde_json::json!("hello"));
  }

  #[test]
  fn datetime_string_field_passes_through_for_chrono() {
    #[derive(Deserialize)]
    struct D {
      when: chrono::DateTime<chrono::FixedOffset>,
    }
    let d: D = from_json_value(r#"{"when":"2020-01-02T03:04:05+00:00"}"#).unwrap();
    assert_eq!(d.when.to_rfc3339(), "2020-01-02T03:04:05+00:00");
  }

  #[test]
  fn bad_number_is_a_config_error_not_a_panic() {
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct D {
      n: i64,
    }
    let e = from_json_value::<D>(r#"{"n":"not-a-number"}"#)
      .err()
      .expect("expected coercion error");
    assert!(e.to_string().contains('n') || e.to_string().contains("integer"));
  }

  fn err<T: de::DeserializeOwned>(json: &str) -> String {
    from_json_value::<T>(json).err().expect("expected a config error").to_string()
  }

  #[derive(Deserialize, Debug, PartialEq)]
  struct Widths {
    a: i8,
    b: i16,
    c: i32,
    d: u8,
    e: u16,
    f: u32,
    g: u64,
    h: f32,
    ch: char,
  }

  #[test]
  fn every_integer_and_float_width_coerces_from_strings() {
    let json = r#"{"a":"-1","b":"-2","c":"-3","d":"4","e":"5","f":"6","g":"7","h":"1.5","ch":"x"}"#;
    let w: Widths = from_json_value(json).unwrap();
    assert_eq!(
      w,
      Widths {
        a: -1,
        b: -2,
        c: -3,
        d: 4,
        e: 5,
        f: 6,
        g: 7,
        h: 1.5,
        ch: 'x',
      }
    );
  }

  #[test]
  fn native_json_numbers_bind_to_unsigned_and_float_fields() {
    let json = r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":6,"g":7,"h":2.5,"ch":"y"}"#;
    let w: Widths = from_json_value(json).unwrap();
    assert_eq!(w.g, 7);
    assert!((w.h - 2.5).abs() < f32::EPSILON);
  }

  #[test]
  fn integer_coercion_errors_name_the_problem() {
    assert!(err::<i64>("1.5").contains("expected integer, got 1.5"));
    assert!(err::<i64>("true").contains("expected integer, got true"));
  }

  #[test]
  fn unsigned_coercion_errors_name_the_problem() {
    assert!(err::<u64>(r#""-3""#).contains("expected unsigned integer"));
    assert!(err::<u64>("-3").contains("expected unsigned integer, got -3"));
    assert!(err::<u64>("[]").contains("expected unsigned integer, got []"));
  }

  #[test]
  fn float_coercion_errors_name_the_problem() {
    assert!((from_json_value::<f64>("4").unwrap() - 4.0).abs() < f64::EPSILON);
    assert!(err::<f64>(r#""abc""#).contains("expected number"));
    assert!(err::<f64>("null").contains("expected number, got null"));
  }

  #[test]
  fn bool_coercion_rejects_other_strings() {
    assert!(err::<bool>(r#""yes""#).contains("expected bool"));
  }

  #[test]
  fn string_field_rejects_non_strings() {
    assert!(err::<String>("5").contains("expected string, got 5"));
  }

  #[test]
  fn config_error_display_is_prefixed() {
    let e = from_json_value::<i64>("true").expect_err("error");
    assert_eq!(e.to_string(), "config error: expected integer, got true");
  }

  #[test]
  fn invalid_json_text_is_a_config_error() {
    assert!(err::<i64>("{not json").starts_with("config error:"));
  }

  #[test]
  fn missing_required_field_uses_the_custom_error() {
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Required {
      name: String,
    }
    assert!(err::<Required>("{}").contains("missing field `name`"));
  }

  #[test]
  fn unit_and_unit_struct_accept_only_null() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Marker;
    from_json_value::<()>("null").unwrap();
    assert_eq!(from_json_value::<Marker>("null").unwrap(), Marker);
    assert!(err::<()>(r#""x""#).contains("expected null"));
  }

  #[test]
  fn newtype_struct_unwraps_its_inner_value() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Port(u16);
    assert_eq!(from_json_value::<Port>(r#""80""#).unwrap(), Port(80));
  }

  #[test]
  fn tuples_and_tuple_structs_read_arrays() {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Pair(i64, bool);
    assert_eq!(
      from_json_value::<(i64, String)>(r#"["1","a"]"#).unwrap(),
      (1, "a".to_string())
    );
    assert_eq!(from_json_value::<Pair>(r#"["2","true"]"#).unwrap(), Pair(2, true));
    assert!(err::<Vec<i64>>(r#""x""#).contains("expected array"));
  }

  #[test]
  fn maps_and_structs_require_objects() {
    let m: std::collections::BTreeMap<String, i64> = from_json_value(r#"{"a":"1","b":"2"}"#).unwrap();
    assert_eq!(m.get("b"), Some(&2));
    assert!(err::<Cfg>("[]").contains("expected object"));
  }

  #[test]
  fn unknown_struct_fields_are_ignored() {
    let cfg: Cfg = from_json_value(r#"{"port":"1","extra":{"deep":["x"]}}"#).unwrap();
    assert_eq!(cfg.port, 1);
  }

  #[derive(Deserialize, Debug, PartialEq)]
  enum Mode {
    Fast,
    Level(u8),
    Range(i64, i64),
    Named { depth: u32 },
  }

  #[test]
  fn enums_bind_every_variant_shape() {
    assert_eq!(from_json_value::<Mode>(r#""Fast""#).unwrap(), Mode::Fast);
    assert_eq!(from_json_value::<Mode>(r#"{"Level":"3"}"#).unwrap(), Mode::Level(3));
    assert_eq!(from_json_value::<Mode>(r#"{"Range":["1","2"]}"#).unwrap(), Mode::Range(1, 2));
    assert_eq!(
      from_json_value::<Mode>(r#"{"Named":{"depth":"4"}}"#).unwrap(),
      Mode::Named { depth: 4 }
    );
    assert_eq!(from_json_value::<Mode>(r#"{"Fast":null}"#).unwrap(), Mode::Fast);
  }

  #[test]
  fn enums_reject_other_shapes() {
    assert!(err::<Mode>("5").contains("expected enum"));
    assert!(err::<Mode>(r#"{"Fast":null,"Level":"1"}"#).contains("expected enum"));
    assert!(err::<Mode>(r#""Slow""#).contains("unknown variant"));
  }

  #[test]
  fn dynamic_targets_see_native_json_as_is() {
    let v: serde_json::Value = from_json_value(r#"{"n":null,"b":false,"i":-1,"u":18446744073709551615,"f":0.5}"#).unwrap();
    assert_eq!(
      v,
      serde_json::json!({ "n": null, "b": false, "i": -1, "u": 18_446_744_073_709_551_615_u64, "f": 0.5 })
    );
  }

  #[test]
  fn dynamic_targets_coerce_false_strings_and_arrays() {
    let v: serde_json::Value = from_json_value(r#"["FALSE","7"]"#).unwrap();
    assert_eq!(v, serde_json::json!([false, 7]));
  }

  #[test]
  fn from_serde_value_accepts_parsed_values() {
    let cfg: Cfg = from_serde_value(serde_json::json!({ "label": "x" })).unwrap();
    assert_eq!(cfg.label, "x");
  }
}
