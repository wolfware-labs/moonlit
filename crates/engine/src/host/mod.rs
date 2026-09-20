mod auth;
mod error;
mod imports;
mod net;
pub mod state;
pub mod wit;

use crate::host::state::HostState;
use crate::logging::LogLevel;
use crate::wit::moonlit::plugin::host::add_to_linker as host_add_to_linker;
use crate::wit::moonlit::plugin::process::OutputChunk;
use crate::wit::moonlit::plugin::process::add_to_linker as proc_add_to_linker;
use std::path::PathBuf;
use wasmtime::component::{HasSelf, Linker};
use wasmtime::{Config, Engine};

pub trait HostEventSink: Send + Sync {
    fn log(&self, step: &str, level: LogLevel, message: &str);
    fn progress(&self, step: &str, message: &str);
}

pub struct ChildProc {
    rx: tokio::sync::mpsc::Receiver<OutputChunk>,
    exit_rx: Option<tokio::sync::oneshot::Receiver<i32>>,
    exit_cached: Option<i32>,
    kill_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

pub fn build_engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    #[allow(deprecated)]
    config.async_support(true);
    config.wasm_component_model(true);
    Ok(Engine::new(&config)?)
}

fn build_linker(engine: &Engine) -> anyhow::Result<Linker<HostState>> {
    let mut linker: Linker<HostState> = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
    host_add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    proc_add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    Ok(linker)
}

pub fn test_engine() -> Engine {
    build_engine().expect("engine build")
}
