use crate::expr::Resolve;
use crate::expr::substitute::substitute;
use crate::expr::value::Value;

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
