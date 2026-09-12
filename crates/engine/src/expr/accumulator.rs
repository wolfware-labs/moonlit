//! The configuration layering "accumulator" (§5.2): an ordered stack of `Value::Map` layers.
//! Later layers win. `resolve` walks layers top-down for a single path; `merged` deep-merges a
//! whole top-level section (e.g. `output`) across every layer for condition-scope building.

use indexmap::IndexMap;

use crate::expr::value::Value;

/// Anything that can resolve a `:`-separated path to a value. Implemented by [`Accumulator`];
/// taken by the substitution engine so it can resolve without depending on the concrete stack.
pub trait Resolve {
    fn resolve(&self, path: &str) -> Option<Value>;
}

/// An ordered stack of `Value::Map` layers; later layers win.
#[derive(Debug, Default)]
pub struct Accumulator {
    layers: Vec<Value>,
}

impl Accumulator {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Push a layer (expected to be a `Value::Map` at the top level).
    pub fn push(&mut self, layer: Value) {
        self.layers.push(layer);
    }

    /// Resolve a `:`-separated path, latest layer first. A resolved `Null` leaf counts as absent.
    pub fn resolve(&self, path: &str) -> Option<Value> {
        for layer in self.layers.iter().rev() {
            if let Some(v) = lookup(layer, path) {
                return Some(v);
            }
        }
        None
    }

    /// Deep-merge a top-level section (e.g. `output`) across every layer, later layers winning
    /// per leaf. Maps merge recursively; scalars and lists replace.
    pub fn merged(&self, section: &str) -> Value {
        let mut out: IndexMap<String, Value> = IndexMap::new();
        for layer in &self.layers {
            if let Value::Map(m) = layer
                && let Some(Value::Map(sec)) = m.get(section)
            {
                deep_merge(&mut out, sec);
            }
        }
        Value::Map(out)
    }

    /// Base layer (§5.2.1): `.env` entries (parsed without touching process env) then
    /// `MOONLIT_`-prefixed env vars (prefix stripped), env overriding `.env` on collision.
    pub fn build_base_layer(env: &[(String, String)], dotenv: Option<&str>) -> Value {
        let mut map: IndexMap<String, Value> = IndexMap::new();
        if let Some(contents) = dotenv {
            // `.flatten()` keeps only the Ok pairs; iterating does NOT touch process env.
            for (k, v) in dotenvy::from_read_iter(contents.as_bytes()).flatten() {
                map.insert(k, Value::Str(v));
            }
        }
        for (k, v) in env {
            if let Some(stripped) = k.strip_prefix("MOONLIT_") {
                map.insert(stripped.to_string(), Value::Str(v.clone()));
            }
        }
        Value::Map(map)
    }

    /// Release layer (§5.2.2): `{ vars: {...}, args: {...} }`; CLI args override YAML args.
    pub fn build_release_layer(
        vars: &IndexMap<String, String>,
        args: &IndexMap<String, String>,
        cli_args: &[(String, String)],
    ) -> Value {
        let to_map = |src: &IndexMap<String, String>| {
            Value::Map(
                src.iter()
                    .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                    .collect(),
            )
        };
        let mut args_map = match to_map(args) {
            Value::Map(m) => m,
            _ => unreachable!(),
        };
        for (k, v) in cli_args {
            args_map.insert(k.clone(), Value::Str(v.clone()));
        }
        let mut root = IndexMap::new();
        root.insert("vars".to_string(), to_map(vars));
        root.insert("args".to_string(), Value::Map(args_map));
        Value::Map(root)
    }
}

impl Resolve for Accumulator {
    fn resolve(&self, path: &str) -> Option<Value> {
        Accumulator::resolve(self, path)
    }
}

fn lookup(root: &Value, path: &str) -> Option<Value> {
    let mut cur = root;
    for seg in path.split(':') {
        match cur {
            Value::Map(m) => cur = m.get(seg)?,
            Value::List(l) => {
                let i: usize = seg.parse().ok()?;
                cur = l.get(i)?;
            }
            _ => return None,
        }
    }
    match cur {
        Value::Null => None,
        other => Some(other.clone()),
    }
}

fn deep_merge(dst: &mut IndexMap<String, Value>, src: &IndexMap<String, Value>) {
    for (k, v) in src {
        match (dst.get_mut(k), v) {
            (Some(Value::Map(d)), Value::Map(s)) => deep_merge(d, s),
            _ => {
                dst.insert(k.clone(), v.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(pairs: Vec<(&str, Value)>) -> Value {
        Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }
    fn imap(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn later_layers_win() {
        let mut acc = Accumulator::new();
        acc.push(layer(vec![("k", s("first"))]));
        acc.push(layer(vec![("k", s("second"))]));
        assert_eq!(acc.resolve("k"), Some(s("second")));
    }

    #[test]
    fn resolves_nested_path_with_list_index() {
        let mut acc = Accumulator::new();
        acc.push(layer(vec![(
            "output",
            layer(vec![(
                "commits",
                layer(vec![(
                    "details",
                    Value::List(vec![layer(vec![("sha", s("abc"))])]),
                )]),
            )]),
        )]));
        assert_eq!(acc.resolve("output:commits:details:0:sha"), Some(s("abc")));
        assert_eq!(acc.resolve("output:commits:missing"), None);
    }

    #[test]
    fn null_leaf_resolves_as_absent() {
        let mut acc = Accumulator::new();
        acc.push(layer(vec![("k", Value::Null)]));
        assert_eq!(acc.resolve("k"), None);
    }

    #[test]
    fn merged_deep_merges_section_across_layers() {
        let mut acc = Accumulator::new();
        acc.push(layer(vec![(
            "output",
            layer(vec![("a", layer(vec![("x", s("1"))]))]),
        )]));
        acc.push(layer(vec![(
            "output",
            layer(vec![("b", layer(vec![("y", s("2"))]))]),
        )]));
        let merged = acc.merged("output");
        assert_eq!(
            merged,
            layer(vec![
                ("a", layer(vec![("x", s("1"))])),
                ("b", layer(vec![("y", s("2"))])),
            ])
        );
    }

    #[test]
    fn base_layer_strips_prefix_and_env_overrides_dotenv() {
        let env = vec![
            ("MOONLIT_TOKEN".to_string(), "from-env".to_string()),
            ("PATH".to_string(), "ignored".to_string()),
        ];
        let dotenv = "TOKEN=from-file\nGREETING=hello\n";
        let layer = Accumulator::build_base_layer(&env, Some(dotenv));
        let mut acc = Accumulator::new();
        acc.push(layer);
        assert_eq!(acc.resolve("TOKEN"), Some(s("from-env"))); // env wins over .env
        assert_eq!(acc.resolve("GREETING"), Some(s("hello")));
        assert_eq!(acc.resolve("PATH"), None); // no MOONLIT_ prefix
    }

    #[test]
    fn release_layer_cli_overrides_yaml_args() {
        let vars = imap(&[("channel", "stable")]);
        let args = imap(&[("skipPush", "false"), ("tag", "v1")]);
        let cli = vec![("skipPush".to_string(), "true".to_string())];
        let layer = Accumulator::build_release_layer(&vars, &args, &cli);
        let mut acc = Accumulator::new();
        acc.push(layer);
        assert_eq!(acc.resolve("vars:channel"), Some(s("stable")));
        assert_eq!(acc.resolve("args:tag"), Some(s("v1")));
        assert_eq!(acc.resolve("args:skipPush"), Some(s("true"))); // CLI wins
    }
}
