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
