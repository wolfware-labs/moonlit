//! Pipeline configuration parsing (§4): YAML → validated [`PipelineConfig`].
//!
//! Grows across Phase 2 tasks: model → diagnostic → tree → convert → cleanup/validate/wiring.

pub mod diagnostic;
pub mod model;
