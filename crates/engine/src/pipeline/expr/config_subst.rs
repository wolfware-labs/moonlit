use crate::pipeline::config::{ConfigMap, ConfigValue};
use crate::pipeline::expr::Resolve;
use crate::pipeline::expr::substitute::substitute;
use crate::pipeline::expr::value::Value;

#[must_use]
pub fn substitute_config(config: &ConfigMap, resolver: &dyn Resolve) -> Value {
  Value::Map(
    config
      .iter()
      .map(|(k, sv)| (k.clone(), subst_value(&sv.value, resolver)))
      .collect(),
  )
}

#[must_use]
fn subst_value(v: &ConfigValue, resolver: &dyn Resolve) -> Value {
  match v {
    ConfigValue::Null => Value::Null,
    ConfigValue::String(s) => substitute(s, resolver),
    ConfigValue::List(items) => Value::List(items.iter().map(|sv| subst_value(&sv.value, resolver)).collect()),
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
  use crate::pipeline::config::parse_config;
  use indexmap::IndexMap;

  struct Version;

  impl Resolve for Version {
    fn resolve(&self, path: &str) -> Option<Value> {
      (path == "args:version").then(|| Value::Str("1.2".to_string()))
    }
  }

  #[test]
  fn substitutes_through_nested_config_values() {
    let yaml = "plugins:\n  - name: tp\n    url: file://tp.wasm\n    config:\n      tag: v$(args:version)\n      empty: ~\n      list: [$(args:version), fixed]\n      nested:\n        inner: $(args:version)\nstages:\n  b:\n    - run: tp.go\n";
    let config = parse_config(yaml, "release.yml").expect("valid configuration");
    let value = substitute_config(&config.plugins.value[0].config, &Version);

    let version = || Value::Str("1.2".to_string());
    assert_eq!(
      value,
      Value::Map(IndexMap::from([
        ("tag".to_string(), Value::Str("v1.2".to_string())),
        ("empty".to_string(), Value::Null),
        (
          "list".to_string(),
          Value::List(vec![version(), Value::Str("fixed".to_string())]),
        ),
        (
          "nested".to_string(),
          Value::Map(IndexMap::from([("inner".to_string(), version())])),
        ),
      ]))
    );
  }
}
