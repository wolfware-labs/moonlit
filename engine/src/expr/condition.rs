//! `condition` / `haltIf` evaluation (§5.3) on top of `rhai`. Only the `output` variable is
//! exposed, as a coerced map with lowercase key aliases; identifiers are lowercased by a
//! string-literal-safe pre-pass so member access is case-insensitive. A `condition` that errors
//! warns and yields `false`; a `haltIf` that errors fails (deliberate deviation, Appendix B).

use chrono::{DateTime, FixedOffset};
use miette::Diagnostic;
use rhai::{Array, Dynamic, Engine, Map as RhaiMap, Scope};
use thiserror::Error;

use crate::expr::accumulator::Accumulator;
use crate::expr::coerce::{Scalar, coerce};
use crate::expr::value::Value;

/// The result of evaluating a `condition` (never fails — errors degrade to `false` + a warning).
pub struct ConditionOutcome {
    pub value: bool,
    pub warning: Option<String>,
}

/// A `haltIf` evaluation failure. Distinct from `config::ConfigDiagnostic`.
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

/// Evaluate a `condition`: any error degrades to `false` with a warning.
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

/// Evaluate a `haltIf`: an error fails with an [`EvalError`].
pub fn evaluate_halt(expr: &str, acc: &Accumulator) -> Result<bool, EvalError> {
    eval(expr, acc).map_err(|message| EvalError {
        message,
        expr: expr.to_string(),
    })
}

/// Compile + evaluate a boolean expression. Non-boolean results are `false`.
///
/// Safety note: this runs the expression through a `rhai` [`Engine`] configured as a sandboxed
/// expression evaluator (see [`build_engine`]) — bounded operations/depth/string/array/map sizes
/// and `eval` (rhai's own dynamic-code builtin) disabled via `disable_symbol("eval")`. There is
/// no host filesystem/network/process access exposed to the expression; it can only read the
/// `output` scope built by [`build_output_scope`]. This is the deliberate `condition`/`haltIf`
/// mechanism (§5.3), not an arbitrary-code `eval()`.
fn eval(expr: &str, acc: &Accumulator) -> Result<bool, String> {
    let engine = build_engine();
    let normalized = normalize_identifiers(expr);
    let mut scope = Scope::new();
    scope.push_constant("output", build_output_scope(acc));
    match engine.eval_expression_with_scope::<Dynamic>(&mut scope, &normalized) {
        Ok(d) => Ok(d.as_bool().unwrap_or(false)),
        Err(e) => Err(e.to_string()),
    }
}

/// A sandboxed rhai engine: bounded operations/depth/sizes, no `eval`, datetime comparisons.
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

/// Build the `output` scope value: the merged `output` section, coerced, with lowercase aliases.
fn build_output_scope(acc: &Accumulator) -> Dynamic {
    value_to_dynamic(&acc.merged("output"))
}

fn value_to_dynamic(v: &Value) -> Dynamic {
    match v {
        Value::Null => Dynamic::UNIT,
        Value::Str(s) => scalar_to_dynamic(coerce(s)),
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

/// Lowercase every character outside string literals, so identifiers/keywords are
/// case-insensitive while `'Main'` / `"Main"` literal contents are preserved verbatim.
fn normalize_identifiers(expr: &str) -> String {
    let mut out = String::with_capacity(expr.len());
    let mut chars = expr.chars();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                out.push(c);
                if c == '\\' {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                } else if c == q {
                    quote = None;
                }
            }
            None => {
                if c == '\'' || c == '"' {
                    quote = Some(c);
                    out.push(c);
                } else {
                    out.extend(c.to_lowercase());
                }
            }
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
        // Mixed-case identifiers resolve; the "main" literal keeps its case. rhai reserves
        // single quotes for single-character literals, so string literals use double quotes.
        assert!(evaluate_condition("output.Repo.Branch == \"main\"", &acc).value);
        assert!(!evaluate_condition("output.repo.branch == \"MAIN\"", &acc).value);
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
        // Exceeds max expression depth -> eval error -> condition false with a warning.
        let out = evaluate_condition(&expr, &acc);
        assert!(!out.value);
        assert!(out.warning.is_some());
    }
}
