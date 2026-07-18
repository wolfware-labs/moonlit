//! Expression & condition engine (§5): `$()` substitution, the layering accumulator, scalar
//! coercion, and `rhai` condition evaluation. Pure mechanisms — no pipeline executor yet.

pub mod value;

pub use value::Value;

pub mod accumulator;

pub use accumulator::{Accumulator, Resolve};

pub mod substitute;

pub use substitute::substitute;
