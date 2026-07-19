//! The public engine facade: construction, options, the error taxonomy, and `load_pipeline`
//! (loader). `run` (the executor loop) lands in Phase 7.

// `EngineError` embeds the large `ConfigDiagnostic` (source text + labels); the spec fixes the
// fallible surface as `Result<_, EngineError>` unboxed, so silence the large-err lint here exactly
// as `config/mod.rs` does.
#![allow(clippy::result_large_err)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::cache::{Cache, SystemClock};

/// Engine-wide settings fixed at construction.
pub struct EngineSettings {
    /// Cache root override; `None` uses the OS cache dir (`<cache>/moonlit`).
    pub cache_dir: Option<PathBuf>,
    /// TTL for cached OCI tag→digest resolutions (§8.3).
    pub tag_ttl: Duration,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            cache_dir: None,
            tag_ttl: Duration::from_secs(15 * 60),
        }
    }
}

/// Per-run options.
pub struct PipelineOptions {
    pub working_directory: PathBuf,
    /// Case-insensitive stage names to run; empty = all stages.
    pub stages_filter: Vec<String>,
    pub cli_args: Vec<(String, String)>,
    /// Consumed by the Phase-7 runner.
    pub step_timeout: Option<Duration>,
    /// Fail instead of pulling on a cache miss.
    pub offline: bool,
}

/// The engine error taxonomy. Exit codes: Config=2, PluginLoad=3, Execution=4, Internal=1 (§7.2).
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum EngineError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Config(#[from] crate::config::ConfigDiagnostic),

    #[error("failed to load plugin '{plugin}': {message}")]
    #[diagnostic(code(moonlit::engine::plugin_load))]
    PluginLoad { plugin: String, message: String },

    // Produced by the Phase-7 runner; declared now to freeze the surface.
    #[error("pipeline execution failed: {0}")]
    #[diagnostic(code(moonlit::engine::execution))]
    Execution(String),

    #[error(transparent)]
    #[diagnostic(code(moonlit::engine::internal))]
    Internal(#[from] anyhow::Error),
}

impl EngineError {
    /// Doc-promised exit-code mapping (§7.2).
    pub fn exit_code(&self) -> i32 {
        match self {
            EngineError::Config(_) => 2,
            EngineError::PluginLoad { .. } => 3,
            EngineError::Execution(_) => 4,
            EngineError::Internal(_) => 1,
        }
    }
}

/// The Moonlit engine: a shared `wasmtime::Engine`, the plugin cache, and settings.
// TODO(Task 4): remove once `load_pipeline` reads `wasmtime`/`cache`/`tag_ttl` to resolve and
// instantiate plugins; all three are dormant until then, which trips clippy's dead-code lint
// under `-D warnings`.
#[allow(dead_code)]
pub struct Engine {
    pub(crate) wasmtime: wasmtime::Engine,
    pub(crate) cache: Arc<Cache>,
    pub(crate) tag_ttl: Duration,
}

impl Engine {
    pub fn new(settings: EngineSettings) -> Result<Self, EngineError> {
        let wasmtime = crate::host::build_engine().map_err(EngineError::Internal)?;
        let cache = match settings.cache_dir {
            Some(dir) => Cache::with_root_and_clock(dir, Box::new(SystemClock)),
            None => Cache::new().map_err(|e| EngineError::Internal(e.into()))?,
        };
        Ok(Self {
            wasmtime,
            cache: Arc::new(cache),
            tag_ttl: settings.tag_ttl,
        })
    }
}
