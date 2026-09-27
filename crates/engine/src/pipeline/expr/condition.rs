use chrono::{DateTime, FixedOffset};
use miette::Diagnostic;
use rhai::{Array, Dynamic, Engine, Map as RhaiMap, Scope};
use thiserror::Error;

use crate::pipeline::PipelineData;
use crate::pipeline::expr::scalar::Scalar;
use crate::pipeline::expr::substitute::substitute_with;
use crate::pipeline::expr::value::Value;

#[must_use = "a condition outcome decides whether the step runs"]
pub struct ConditionOutcome {
  pub value: bool,
  pub warning: Option<String>,
}

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(moonlit::condition))]
pub struct EvalError {
  message: String,
}

impl EvalError {
  #[must_use]
  pub fn message(&self) -> &str {
    &self.message
  }
}

pub fn evaluate_condition(expr: &str, pipeline_data: &PipelineData) -> ConditionOutcome {
  match eval(expr, pipeline_data) {
    Ok(value) => ConditionOutcome { value, warning: None },
    Err(message) => ConditionOutcome {
      value: false,
      warning: Some(message),
    },
  }
}

pub fn evaluate_halt(expr: &str, pipeline_data: &PipelineData) -> Result<bool, EvalError> {
  eval(expr, pipeline_data).map_err(|message| EvalError { message })
}

fn eval(expr: &str, pipeline_data: &PipelineData) -> Result<bool, String> {
  let engine = build_engine();
  let substituted = substitute_condition(expr, pipeline_data);
  let normalized = normalize_identifiers(&substituted);
  let mut scope = Scope::new();
  scope.push_constant("output", build_output_scope(pipeline_data));
  match engine.eval_expression_with_scope::<Dynamic>(&mut scope, &normalized) {
    Ok(d) => Ok(d.as_bool().unwrap_or(false)),
    Err(e) => Err(e.to_string()),
  }
}

#[must_use]
fn substitute_condition(expr: &str, pipeline_data: &PipelineData) -> String {
  substitute_with(expr, pipeline_data, |v| match v {
    Some(value) => value_to_literal(&value),
    None => "''".to_string(),
  })
}

#[must_use]
fn value_to_literal(v: &Value) -> String {
  match v {
    Value::Null => "''".to_string(),
    Value::Str(s) => match Scalar::from(s.as_str()) {
      Scalar::Bool(b) => b.to_string(),
      Scalar::Int(i) => i.to_string(),
      Scalar::Float(f) => f.to_string(),
      Scalar::DateTime(_) => quote_rhai(s),
      Scalar::Str(s) => quote_rhai(&s),
    },
    _ => quote_rhai(&v.to_json_string()),
  }
}

#[must_use]
fn quote_rhai(s: &str) -> String {
  let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
  format!("'{escaped}'")
}

#[must_use]
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
  engine.register_fn("==", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a == b);
  engine.register_fn("!=", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a != b);
  engine.register_fn("<", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a < b);
  engine.register_fn("<=", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a <= b);
  engine.register_fn(">", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a > b);
  engine.register_fn(">=", |a: DateTime<FixedOffset>, b: DateTime<FixedOffset>| a >= b);
}

fn build_output_scope(pipeline_data: &PipelineData) -> Dynamic {
  value_to_dynamic(&pipeline_data.merged("output"))
}

fn value_to_dynamic(v: &Value) -> Dynamic {
  match v {
    Value::Null => Dynamic::UNIT,
    Value::Str(s) => scalar_to_dynamic(Scalar::from(s.as_str())),
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

#[must_use]
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
  use indexmap::IndexMap;

  fn text(s: &str) -> Value {
    Value::Str(s.to_string())
  }

  fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
      entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect::<IndexMap<_, _>>(),
    )
  }

  fn data() -> PipelineData {
    let mut data = PipelineData::default();
    data.push(map(vec![(
      "args",
      map(vec![
        ("version", text("1.5")),
        ("count", text("3")),
        ("flag", text("true")),
        ("region", text("eu")),
        ("when", text("2024-01-02")),
        ("quote", text("it's \\ ok")),
        ("items", Value::List(vec![text("a")])),
      ]),
    )]));
    data.push(map(vec![(
      "output",
      map(vec![(
        "S1",
        map(vec![
          ("Step", text("s1")),
          ("done", text("TRUE")),
          ("count", text("2")),
          ("ratio", text("0.5")),
          ("early", text("2024-01-01")),
          ("late", text("2024-06-01T10:00:00Z")),
          ("nothing", Value::Null),
          ("items", Value::List(vec![text("a"), text("b")])),
        ]),
      )]),
    )]));
    data
  }

  fn holds(expr: &str) -> bool {
    let outcome = evaluate_condition(expr, &data());
    assert_eq!(outcome.warning, None, "{expr}");
    outcome.value
  }

  #[test]
  fn literal_booleans_evaluate_directly() {
    assert!(holds("true"));
    assert!(!holds("false"));
  }

  #[test]
  fn placeholders_become_typed_literals() {
    assert!(holds("$(args:version) > 1.0"));
    assert!(holds("$(args:count) == 3"));
    assert!(holds("$(args:flag)"));
    assert!(holds("$(args:region) == 'eu'"));
    assert!(holds("$(args:when) == '2024-01-02'"));
    assert!(holds("$(args:items) == '[\"a\"]'"));
  }

  #[test]
  fn missing_placeholders_become_empty_strings() {
    assert!(holds("$(missing) == ''"));
  }

  #[test]
  fn quotes_and_backslashes_in_values_are_escaped() {
    assert!(holds(r"$(args:quote) == 'it\'s \\ ok'"));
  }

  #[test]
  fn output_scope_is_case_insensitive() {
    assert!(holds("output.S1.Step == 's1'"));
    assert!(holds("output.s1.step == 's1'"));
  }

  #[test]
  fn output_scope_values_are_typed() {
    assert!(holds("output.s1.done"));
    assert!(holds("output.s1.count + 1 == 3"));
    assert!(holds("output.s1.ratio < 1.0"));
    assert!(holds("output.s1.items.len() == 2"));
    assert!(holds("output.s1.nothing == ()"));
  }

  #[test]
  fn datetimes_compare_with_every_operator() {
    assert!(holds(
      "output.s1.early < output.s1.late && output.s1.early <= output.s1.late && output.s1.late > output.s1.early && output.s1.late >= output.s1.early && output.s1.early != output.s1.late && output.s1.early == output.s1.early"
    ));
  }

  #[test]
  fn non_boolean_results_are_false() {
    assert!(!holds("1 + 1"));
  }

  #[test]
  fn invalid_conditions_are_false_with_a_warning() {
    let outcome = evaluate_condition("this is not @ valid", &data());
    assert!(!outcome.value);
    assert!(outcome.warning.is_some());
  }

  #[test]
  fn eval_is_disabled() {
    let outcome = evaluate_condition("eval(\"true\")", &data());
    assert!(!outcome.value);
    assert!(outcome.warning.is_some());
  }

  #[test]
  fn halt_evaluates_like_a_condition() {
    assert!(evaluate_halt("output.s1.done", &data()).unwrap());
    assert!(!evaluate_halt("false", &data()).unwrap());
  }

  #[test]
  fn halt_reports_evaluation_errors() {
    let error = evaluate_halt("this is not @ valid", &data()).unwrap_err();
    assert!(!error.message().is_empty());
    assert_eq!(error.to_string(), error.message());
  }

  #[test]
  fn value_literals_cover_every_value_kind() {
    assert_eq!(value_to_literal(&Value::Null), "''");
    assert_eq!(value_to_literal(&text("false")), "false");
    assert_eq!(value_to_literal(&text("7")), "7");
    assert_eq!(value_to_literal(&text("2.5")), "2.5");
    assert_eq!(value_to_literal(&text("2024-01-02")), "'2024-01-02'");
    assert_eq!(value_to_literal(&text("main")), "'main'");
    assert_eq!(value_to_literal(&map(vec![("k", text("v"))])), r#"'{"k":"v"}'"#);
  }

  #[test]
  fn identifiers_are_lowercased_outside_strings() {
    assert_eq!(normalize_identifiers("Output.S1 == 'MiXeD'"), "output.s1 == \"MiXeD\"");
    assert_eq!(normalize_identifiers("A == \"KeEp\""), "a == \"KeEp\"");
  }

  #[test]
  fn single_quoted_escapes_are_translated() {
    assert_eq!(normalize_identifiers(r"'it\'s'"), "\"it's\"");
    assert_eq!(normalize_identifiers(r#"'say \"hi\"'"#), r#""say \"hi\"""#);
    assert_eq!(normalize_identifiers(r#"'a"b'"#), r#""a\"b""#);
    assert_eq!(normalize_identifiers(r"'a\nb'"), r#""a\nb""#);
    assert_eq!(normalize_identifiers(r"'trailing\"), "\"trailing\\");
  }

  #[test]
  fn double_quoted_escapes_are_kept() {
    assert_eq!(normalize_identifiers(r#""a\"B" == X"#), r#""a\"B" == x"#);
    assert_eq!(normalize_identifiers("\"end\\"), "\"end\\");
  }
}
