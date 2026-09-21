mod condition;
mod config_subst;
mod scalar;
mod substitute;
mod value;

use crate::expr::value::Value;

pub trait Resolve {
    fn resolve(&self, path: &str) -> Option<Value>;
}
