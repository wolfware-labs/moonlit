//! Recursive `$()` substitution over a parsed config tree (§5.1/§5.2). Bridges the `config` model
//! into the substitution engine: each scalar string is run through `substitute`, structure preserved.

use crate::config::model::{ConfigMap, ConfigValue};
use crate::expr::accumulator::Resolve;
use crate::expr::substitute::substitute;
use crate::expr::value::Value;

/// Substitute every scalar in `config` against `resolver`, preserving map/list structure.
pub fn substitute_config(config: &ConfigMap, resolver: &dyn Resolve) -> Value {
    Value::Map(
        config
            .iter()
            .map(|(k, sv)| (k.clone(), subst_value(&sv.value, resolver)))
            .collect(),
    )
}

fn subst_value(v: &ConfigValue, resolver: &dyn Resolve) -> Value {
    match v {
        ConfigValue::Null => Value::Null,
        ConfigValue::String(s) => substitute(s, resolver),
        ConfigValue::List(items) => Value::List(
            items
                .iter()
                .map(|sv| subst_value(&sv.value, resolver))
                .collect(),
        ),
        ConfigValue::Map(m) => Value::Map(
            m.iter()
                .map(|(k, sv)| (k.clone(), subst_value(&sv.value, resolver)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{ConfigValue, Span, Spanned};
    use crate::expr::accumulator::Accumulator;
    use crate::expr::value::Value;
    use indexmap::IndexMap;

    fn cmap(pairs: Vec<(&str, ConfigValue)>) -> ConfigMap {
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), Spanned::new(v, Span::new(0, 0))))
            .collect()
    }

    fn acc_with(layer: Value) -> Accumulator {
        let mut a = Accumulator::new();
        a.push(layer);
        a
    }

    #[test]
    fn embedded_substitution_yields_string() {
        // resolver: { vars: { x: "1" } }
        let layer = Value::Map(IndexMap::from([(
            "vars".to_string(),
            Value::Map(IndexMap::from([(
                "x".to_string(),
                Value::Str("1".to_string()),
            )])),
        )]));
        let cfg = cmap(vec![("a", ConfigValue::String("v$(vars:x)".to_string()))]);
        let out = substitute_config(&cfg, &acc_with(layer));
        assert_eq!(
            out,
            Value::Map(IndexMap::from([(
                "a".to_string(),
                Value::Str("v1".to_string())
            )]))
        );
    }

    #[test]
    fn whole_string_substitution_yields_structure() {
        // resolver: { obj: { k: "v" } }; config a: "$(obj)" -> the whole map
        let inner = Value::Map(IndexMap::from([(
            "k".to_string(),
            Value::Str("v".to_string()),
        )]));
        let layer = Value::Map(IndexMap::from([("obj".to_string(), inner.clone())]));
        let cfg = cmap(vec![("a", ConfigValue::String("$(obj)".to_string()))]);
        let out = substitute_config(&cfg, &acc_with(layer));
        assert_eq!(out, Value::Map(IndexMap::from([("a".to_string(), inner)])));
    }

    #[test]
    fn null_and_nested_structure_preserved() {
        let cfg = cmap(vec![
            ("n", ConfigValue::Null),
            (
                "list",
                ConfigValue::List(vec![Spanned::new(
                    ConfigValue::String("plain".to_string()),
                    Span::new(0, 0),
                )]),
            ),
        ]);
        let out = substitute_config(&cfg, &acc_with(Value::Map(IndexMap::new())));
        assert_eq!(
            out,
            Value::Map(IndexMap::from([
                ("n".to_string(), Value::Null),
                (
                    "list".to_string(),
                    Value::List(vec![Value::Str("plain".to_string())])
                ),
            ]))
        );
    }
}
