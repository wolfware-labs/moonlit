//! The runtime value currency for the expression engine: an unspanned `Null | Str | List | Map`
//! tree (distinct from `config::ConfigValue`, whose children carry YAML spans). Scalars are kept
//! as raw strings; typing happens later in `coerce`. Provides lossless flatten/unflatten (§5.2)
//! and string/JSON rendering for embedded substitution (§5.1).

use indexmap::IndexMap;

/// A dynamic runtime value produced by substitution and stored in accumulator layers.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Str(String),
    List(Vec<Value>),
    Map(IndexMap<String, Value>),
}

impl Value {
    /// Flatten scalar leaves into `:`-joined keys with numeric list indices. `Null` leaves and
    /// empty containers are omitted (a missing key resolves to null anyway).
    pub fn flatten(&self) -> IndexMap<String, String> {
        let mut out = IndexMap::new();
        flatten_into(self, "", &mut out);
        out
    }

    /// Rebuild a tree from flattened keys. The root is a map; a segment of all-ASCII-digits builds
    /// a list, otherwise a map. Lossless for maps/lists of string leaves.
    pub fn unflatten(flat: &IndexMap<String, String>) -> Value {
        let mut root = Value::Map(IndexMap::new());
        for (key, val) in flat {
            let segments: Vec<&str> = key.split(':').collect();
            if segments.is_empty() {
                continue;
            }
            insert_path(&mut root, &segments, val);
        }
        root
    }

    /// The embedded-substitution string form: scalar text, empty for null, JSON for structures.
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Str(s) => s.clone(),
            _ => self.to_json_string(),
        }
    }

    /// Minimal JSON rendering with proper string escaping.
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

fn is_index(seg: &str) -> bool {
    !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit())
}

fn flatten_into(v: &Value, prefix: &str, out: &mut IndexMap<String, String>) {
    match v {
        Value::Null => {}
        Value::Str(s) => {
            if !prefix.is_empty() {
                out.insert(prefix.to_string(), s.clone());
            }
        }
        Value::List(items) => {
            for (i, item) in items.iter().enumerate() {
                let key = if prefix.is_empty() {
                    i.to_string()
                } else {
                    format!("{prefix}:{i}")
                };
                flatten_into(item, &key, out);
            }
        }
        Value::Map(m) => {
            for (k, item) in m {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}:{k}")
                };
                flatten_into(item, &key, out);
            }
        }
    }
}

fn insert_path(parent: &mut Value, segments: &[&str], val: &str) {
    let key = segments[0];
    if segments.len() == 1 {
        put_child(parent, key, Value::Str(val.to_string()));
        return;
    }
    let want_list = is_index(segments[1]);
    let child = child_slot(parent, key, want_list);
    insert_path(child, &segments[1..], val);
}

fn put_child(parent: &mut Value, key: &str, value: Value) {
    match parent {
        Value::Map(m) => {
            m.insert(key.to_string(), value);
        }
        Value::List(l) => {
            let i: usize = key.parse().expect("list index segment");
            while l.len() <= i {
                l.push(Value::Null);
            }
            l[i] = value;
        }
        _ => unreachable!("parent is always a container"),
    }
}

fn child_slot<'a>(parent: &'a mut Value, key: &str, want_list: bool) -> &'a mut Value {
    let fresh = || {
        if want_list {
            Value::List(Vec::new())
        } else {
            Value::Map(IndexMap::new())
        }
    };
    match parent {
        Value::Map(m) => m.entry(key.to_string()).or_insert_with(fresh),
        Value::List(l) => {
            let i: usize = key.parse().expect("list index segment");
            while l.len() <= i {
                l.push(Value::Null);
            }
            if !matches!(l[i], Value::List(_) | Value::Map(_)) {
                l[i] = fresh();
            }
            &mut l[i]
        }
        _ => unreachable!("parent is always a container"),
    }
}

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
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: Vec<(&str, Value)>) -> Value {
        Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }

    #[test]
    fn flatten_unflatten_round_trip_maps_and_lists() {
        let v = map(vec![
            ("name", s("demo")),
            (
                "commits",
                map(vec![(
                    "details",
                    Value::List(vec![
                        map(vec![("sha", s("abc"))]),
                        map(vec![("sha", s("def"))]),
                    ]),
                )]),
            ),
        ]);
        let flat = v.flatten();
        assert_eq!(flat["name"], "demo");
        assert_eq!(flat["commits:details:0:sha"], "abc");
        assert_eq!(flat["commits:details:1:sha"], "def");
        assert_eq!(Value::unflatten(&flat), v);
    }

    #[test]
    fn flatten_omits_null_leaves() {
        let v = map(vec![("a", s("x")), ("b", Value::Null)]);
        let flat = v.flatten();
        assert_eq!(flat.get("a").map(String::as_str), Some("x"));
        assert!(!flat.contains_key("b"));
    }

    #[test]
    fn display_string_renders_scalar_null_and_structured() {
        assert_eq!(s("hi").to_display_string(), "hi");
        assert_eq!(Value::Null.to_display_string(), "");
        let structured = map(vec![("k", s("v"))]);
        assert_eq!(structured.to_display_string(), r#"{"k":"v"}"#);
    }

    #[test]
    fn json_string_escapes_quotes_and_backslashes() {
        assert_eq!(s(r#"a"b\c"#).to_json_string(), r#""a\"b\\c""#);
        let list = Value::List(vec![s("x"), Value::Null]);
        assert_eq!(list.to_json_string(), r#"["x",null]"#);
    }
}
