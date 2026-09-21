use chrono::{DateTime, FixedOffset};
use miette::Diagnostic;
use rhai::{Array, Dynamic, Engine, Map as RhaiMap, Scope};
use thiserror::Error;

use crate::expr::scalar::Scalar;
use crate::expr::substitute::substitute_with;
use crate::expr::value::Value;

pub struct ConditionOutcome {
    pub value: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(moonlit::condition))]
pub struct EvalError {
    message: String,
    expr: String,
}

impl EvalError {
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn expr(&self) -> &str {
        &self.expr
    }
}

pub fn evaluate_condition(expr: &str, acc: &Accumulator) -> ConditionOutcome {
    match eval(expr, acc) {
        Ok(value) => ConditionOutcome {
            value,
            warning: None,
        },
        Err(message) => ConditionOutcome {
            value: false,
            warning: Some(message),
        },
    }
}

pub fn evaluate_halt(expr: &str, acc: &Accumulator) -> Result<bool, EvalError> {
    eval(expr, acc).map_err(|message| EvalError {
        message,
        expr: expr.to_string(),
    })
}

fn eval(expr: &str, acc: &Accumulator) -> Result<bool, String> {
    let engine = build_engine();
    let substituted = substitute_condition(expr, acc);
    let normalized = normalize_identifiers(&substituted);
    let mut scope = Scope::new();
    scope.push_constant("output", build_output_scope(acc));
    match engine.eval_expression_with_scope::<Dynamic>(&mut scope, &normalized) {
        Ok(d) => Ok(d.as_bool().unwrap_or(false)),
        Err(e) => Err(e.to_string()),
    }
}

fn substitute_condition(expr: &str, acc: &Accumulator) -> String {
    substitute_with(expr, acc, |v| match v {
        Some(value) => value_to_literal(&value),
        None => "''".to_string(),
    })
}

fn value_to_literal(v: &Value) -> String {
    match v {
        Value::Null => "''".to_string(),
        Value::Str(s) => match s {
            Scalar::Bool(b) => b.to_string(),
            Scalar::Int(i) => i.to_string(),
            Scalar::Float(f) => f.to_string(),
            Scalar::DateTime(_) => quote_rhai(s),
            Scalar::Str(s) => quote_rhai(&s),
        },
        _ => quote_rhai(&v.to_json_string()),
    }
}

fn quote_rhai(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(50_000);
    engine.set_max_expr_depths(64, 64);
    engine.set_max_string_size(16 * 1024);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);
    engine.disable_symbol("eval");
    register_datetime(&mut engine);
    engine
}

fn register_datetime(engine: &mut Engine) {
    engine.register_type_with_name::<DateTime<FixedOffset>>("Datetime");
    engine.register_fn(
        "==",
        |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a == b,
    );
    engine.register_fn(
        "!=",
        |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a != b,
    );
    engine.register_fn("<", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| {
        a < b
    });
    engine.register_fn(
        "<=",
        |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a <= b,
    );
    engine.register_fn(">", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| {
        a > b
    });
    engine.register_fn(
        ">=",
        |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a >= b,
    );
}

fn build_output_scope(acc: &Accumulator) -> Dynamic {
    value_to_dynamic(&acc.merged("output"))
}

fn value_to_dynamic(v: &Value) -> Dynamic {
    match v {
        Value::Null => Dynamic::UNIT,
        Value::Str(s) => scalar_to_dynamic(s.into()),
        Value::List(items) => {
            let arr: Array = items.iter().map(value_to_dynamic).collect();
            Dynamic::from_array(arr)
        }
        Value::Map(m) => {
            let mut map = RhaiMap::new();
            for (k, val) in m {
                let d = value_to_dynamic(val);
                let lower = k.to_lowercase();
                map.insert(k.as_str().into(), d.clone());
                if lower != *k {
                    map.insert(lower.as_str().into(), d);
                }
            }
            Dynamic::from_map(map)
        }
    }
}

fn scalar_to_dynamic(s: Scalar) -> Dynamic {
    match s {
        Scalar::Bool(b) => Dynamic::from(b),
        Scalar::Int(i) => Dynamic::from(i),
        Scalar::Float(f) => Dynamic::from(f),
        Scalar::DateTime(dt) => Dynamic::from(dt),
        Scalar::Str(s) => Dynamic::from(s),
    }
}

fn normalize_identifiers(expr: &str) -> String {
    let mut out = String::with_capacity(expr.len());
    let mut chars = expr.chars();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        match quote {
            Some('\'') => match c {
                '\\' => match chars.next() {
                    Some('\'') => out.push('\''),
                    Some('"') => out.push_str("\\\""),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                },
                '\'' => {
                    out.push('"');
                    quote = None;
                }
                '"' => out.push_str("\\\""),
                other => out.push(other),
            },
            Some(_) => {
                out.push(c);
                if c == '\\' {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                } else if c == '"' {
                    quote = None;
                }
            }
            None => match c {
                '\'' => {
                    out.push('"');
                    quote = Some('\'');
                }
                '"' => {
                    out.push('"');
                    quote = Some('"');
                }
                _ => out.extend(c.to_lowercase()),
            },
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::value::Value;

    fn map(pairs: Vec<(&str, Value)>) -> Value {
        Value::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }
    fn s(v: &str) -> Value {
        Value::Str(v.to_string())
    }

    fn acc_with_output(output: Value) -> Accumulator {
        let mut a = Accumulator::new();
        a.push(map(vec![("output", output)]));
        a
    }

    #[test]
    fn true_condition_is_true() {
        let acc = acc_with_output(map(vec![(
            "version",
            map(vec![("hasNewVersion", s("true"))]),
        )]));
        let out = evaluate_condition("output.version.hasNewVersion", &acc);
        assert!(out.value);
        assert!(out.warning.is_none());
    }

    #[test]
    fn identifiers_are_case_insensitive_but_string_literals_are_not() {
        let acc = acc_with_output(map(vec![("repo", map(vec![("branch", s("main"))]))]));
        // Mixed-case identifiers resolve; the 'main' literal keeps its case.
        assert!(evaluate_condition("output.Repo.Branch == 'main'", &acc).value);
        assert!(!evaluate_condition("output.repo.branch == 'MAIN'", &acc).value);
    }

    #[test]
    fn numeric_coercion_enables_comparison() {
        let acc = acc_with_output(map(vec![("cc", map(vec![("commitCount", s("0"))]))]));
        assert!(evaluate_condition("output.cc.commitCount == 0", &acc).value);
        assert!(!evaluate_condition("output.cc.commitCount > 0", &acc).value);
    }

    #[test]
    fn negation_of_false_is_true() {
        let acc = acc_with_output(map(vec![(
            "version",
            map(vec![("hasNewVersion", s("false"))]),
        )]));
        assert!(evaluate_condition("!output.version.hasNewVersion", &acc).value);
    }

    #[test]
    fn datetime_values_compare() {
        let acc = acc_with_output(map(vec![(
            "build",
            map(vec![("at", s("2024-01-02T03:04:05Z"))]),
        )]));
        assert!(evaluate_condition("output.build.at == output.build.at", &acc).value);
    }

    #[test]
    fn non_boolean_result_is_false() {
        let acc = acc_with_output(map(vec![("n", map(vec![("v", s("5"))]))]));
        assert!(!evaluate_condition("output.n.v", &acc).value); // integer, not bool
    }

    #[test]
    fn condition_error_warns_and_is_false() {
        let acc = acc_with_output(Value::Map(Default::default()));
        let out = evaluate_condition("this is not @ valid", &acc);
        assert!(!out.value);
        assert!(out.warning.is_some());
    }

    #[test]
    fn halt_error_fails() {
        let acc = acc_with_output(Value::Map(Default::default()));
        let err = evaluate_halt("this is not @ valid", &acc).unwrap_err();
        assert_eq!(err.expr(), "this is not @ valid");
        assert!(!err.message().is_empty());
    }

    #[test]
    fn halt_ok_returns_value() {
        let acc = acc_with_output(map(vec![("v", map(vec![("halt", s("true"))]))]));
        assert!(evaluate_halt("output.v.halt", &acc).unwrap());
    }

    #[test]
    fn deeply_nested_expression_is_rejected_by_limits() {
        let acc = acc_with_output(Value::Map(Default::default()));
        let expr = format!("{}true{}", "(".repeat(200), ")".repeat(200));
        let out = evaluate_condition(&expr, &acc);
        assert!(!out.value);
        assert!(out.warning.is_some());
    }

    #[test]
    fn single_and_double_quoted_strings_are_equivalent() {
        let acc = acc_with_output(map(vec![("repo", map(vec![("branch", s("main"))]))]));
        assert!(evaluate_condition("output.repo.branch == 'main'", &acc).value);
        assert!(evaluate_condition("output.repo.branch == \"main\"", &acc).value);
        assert!(!evaluate_condition("output.repo.branch == 'dev'", &acc).value);
    }

    #[test]
    fn substitutes_string_value_as_quoted_literal() {
        let mut acc = Accumulator::new();
        acc.push(map(vec![(
            "output",
            map(vec![("repo", map(vec![("branch", s("main"))]))]),
        )]));
        assert!(evaluate_condition("$(output:repo:branch) == 'main'", &acc).value);
    }

    #[test]
    fn substitutes_bool_value_as_bare_literal() {
        let mut acc = Accumulator::new();
        acc.push(map(vec![("args", map(vec![("skipPush", s("false"))]))]));
        assert!(evaluate_condition("$(args:skipPush) == false", &acc).value);
    }

    #[test]
    fn unresolved_substitution_becomes_empty_string_literal() {
        let acc = acc_with_output(Value::Map(Default::default()));
        let out = evaluate_condition("$(nosuchkey) == 'main'", &acc);
        assert!(!out.value);
        assert!(out.warning.is_none());
    }

    #[test]
    fn condition_substitution_applies_default_fallback() {
        let acc = acc_with_output(Value::Map(Default::default()));
        assert!(evaluate_condition("$(args:missing:main) == 'main'", &acc).value);
    }

    #[test]
    fn literal_quote_in_substituted_value_is_escaped() {
        let mut acc = Accumulator::new();
        acc.push(map(vec![("args", map(vec![("msg", s("it's"))]))]));
        assert!(evaluate_condition("$(args:msg) == 'it\\'s'", &acc).value);
    }
}
