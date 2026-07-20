//! # moonlit-engine
//!
//! The Moonlit engine: a library that parses release pipelines, resolves and instantiates
//! WebAssembly plugin components (wasmtime, WASI Preview 2), and runs stages and steps to completion.
//!
//! The canonical plugin ABI is authored in `engine/wit/moonlit-plugin.wit`
//! (`moonlit:plugin@0.1.0`). Dynamic config and output values cross that ABI as JSON text
//! (`json-value`) and are bridged to a typed value tree on the Rust side.

pub mod cache;
pub mod config;
mod engine;
pub mod expr;
pub mod host;
pub mod pipeline;
pub mod resolve;

pub use engine::{Engine, EngineError, EngineSettings, PipelineOptions};
pub use host::LogLevel;
pub use pipeline::{Pipeline, PipelineEvent, PipelineSummary, StepResult};
