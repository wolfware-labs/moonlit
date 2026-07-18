//! Pipeline configuration parsing (§4): YAML → validated [`PipelineConfig`].
//!
//! Grows across Phase 2 tasks: model → diagnostic → tree → convert → cleanup/validate/wiring.

// The spec (§7.2) fixes the fallible surface as `Result<_, ConfigDiagnostic>` (unboxed).
// `ConfigDiagnostic` carries the source text + labels and so trips clippy's
// `result_large_err`; boxing it would contradict the mandated signature and add indirection
// on the cold error path. We deliberately keep the unboxed error and silence the lint here.
#![allow(clippy::result_large_err)]

pub mod diagnostic;
pub mod model;
// `tree` is internal and currently consumed only by its own tests; Task 4 (`convert`) makes it
// used in the library build, at which point this `allow(dead_code)` is removed.
#[allow(dead_code)]
mod tree;
