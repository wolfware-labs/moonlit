mod condition;
mod config_subst;
mod scalar;
mod substitute;
mod value;

pub use crate::pipeline::expr::condition::{evaluate_condition, evaluate_halt};
pub use crate::pipeline::expr::config_subst::substitute_config;
pub use crate::pipeline::expr::value::Value;

pub trait Resolve {
  fn resolve(&self, path: &str) -> Option<Value>;
}
