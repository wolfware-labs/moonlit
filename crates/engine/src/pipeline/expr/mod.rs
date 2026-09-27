mod condition;
mod config_subst;
mod scalar;
mod substitute;
mod value;

pub use crate::pipeline::expr::value::Value;

pub trait Resolve {
  fn resolve(&self, path: &str) -> Option<Value>;
}
