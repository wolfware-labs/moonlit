//! # moonlit-engine
//!
//! The Moonlit 2.0 engine: a library that parses release pipelines, resolves and
//! instantiates WebAssembly plugin components (wasmtime, WASI Preview 2), and runs
//! stages and steps to completion.
//!
//! The canonical plugin ABI is authored in `engine/wit/moonlit-plugin.wit`
//! (`moonlit:plugin@2.0.0`). Dynamic config and output values cross that ABI as JSON
//! text (`json-value`) and are bridged to a typed value tree on the Rust side.
//!
//! Module layout grows as milestones land:
//! - `config` — YAML model, deserializers, cleanup, validation (Phase 2)
//! - `expr`   — `$()` substitution, layering, coercion, `rhai` conditions (Phase 3)
//! - `resolve` / `host` / `pipeline` / `cache` — wasm host + executor (M2+)
