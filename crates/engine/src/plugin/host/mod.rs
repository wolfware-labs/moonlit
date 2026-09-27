mod auth;
mod child_process;
// pub mod error;
mod net;
pub mod state;

use crate::logging::LogLevel;
use crate::plugin::host::state::HostState;
use crate::plugin::wit;
pub use auth::{build_wasi_ctx, exec_globset};
pub use child_process::ChildProcess;
pub use net::AllowlistHooks;
use wasmtime::component::{HasSelf, Linker};

pub(crate) fn link(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
  wasmtime_wasi::p2::add_to_linker_async(linker)?;
  wasmtime_wasi_http::p2::add_only_http_to_linker_async(linker)?;
  wit::moonlit::plugin::host::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
  wit::moonlit::plugin::process::add_to_linker::<_, HasSelf<_>>(linker, |s| s)?;
  Ok(())
}

pub trait HostEventSink: Send + Sync {
  fn log(&self, step: &str, level: LogLevel, message: &str);
  fn progress(&self, step: &str, message: &str);
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseContext {
  pub working_directory: String,
  pub step_name: String,
}
