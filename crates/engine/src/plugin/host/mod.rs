mod auth;
mod child_process;
// pub mod error;
mod net;
pub mod state;

use crate::logging::LogLevel;
pub use auth::{build_wasi_ctx, exec_globset};
pub use child_process::ChildProcess;
pub use net::AllowlistHooks;

pub trait HostEventSink: Send + Sync {
  fn log(&self, step: &str, level: LogLevel, message: &str);
  fn progress(&self, step: &str, message: &str);
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseContext {
  pub working_directory: String,
  pub step_name: String,
}
