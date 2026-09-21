use indexmap::IndexMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Str(String),
    List(Vec<Value>),
    Map(IndexMap<String, Value>),
}

impl Value {
    pub fn flatten(&self) -> IndexMap<String, String> {
        let mut out = IndexMap::new();
        flatten_into(self, "", &mut out);
        out
    }

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

    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Str(s) => s.clone(),
            _ => self.to_json_string(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::List(items) => {
                serde_json::Value::Array(items.iter().map(Value::to_json).collect())
            }
            Value::Map(m) => {
                serde_json::Value::Object(m.iter().map(|(k, v)| (k.clone(), v.to_json())).collect())
            }
        }
    }

    pub fn from_json(j: &serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Str(b.to_string()),
            serde_json::Value::Number(n) => Value::Str(n.to_string()),
            serde_json::Value::String(s) => Value::Str(s.clone()),
            serde_json::Value::Array(a) => Value::List(a.iter().map(Value::from).collect()),
            serde_json::Value::Object(o) => Value::Map(
                o.iter()
                    .map(|(k, v)| (k.clone(), Value::from_json(v)))
                    .collect(),
            ),
        }
    }

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
