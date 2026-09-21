mod auth;
mod child_process;
pub mod error;
mod net;
pub mod state;
pub mod wit;

use crate::logging::LogLevel;
pub use child_process::ChildProc;
use wasmtime::{Config, Engine};

pub trait HostEventSink: Send + Sync {
    fn log(&self, step: &str, level: LogLevel, message: &str);
    fn progress(&self, step: &str, message: &str);
}

pub fn build_engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    #[allow(deprecated)]
    config.async_support(true);
    config.wasm_component_model(true);
    Ok(Engine::new(&config)?)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseContext {
    pub working_directory: String,
    pub step_name: String,
}
