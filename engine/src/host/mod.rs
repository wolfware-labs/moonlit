//! wasmtime component host for `moonlit:plugin@2.0.0` (Phase 5): instantiate one
//! resolved plugin and call its exports, with the full host ABI + permission
//! enforcement. bindgen wiring and the `PluginInstance` seam arrive in later tasks.

mod convert;
mod perms;

use std::path::PathBuf;

pub use convert::{HostError, json_str_to_value, value_to_json};
pub use convert::{LogLevel, MiddlewareInfo, MiddlewareResult, PluginMetadata, ReleaseContext};

/// Everything needed to instantiate one plugin (Phase 6 fills these from the run).
pub struct InstanceConfig {
    pub working_directory: PathBuf,
    pub permissions: crate::config::model::Permissions,
    /// Accumulated, `$()`-substituted config served to the guest via `get-config`.
    pub config_view: serde_json::Value,
    /// Process env to be filtered by the `env` grant before the guest sees it.
    pub env_snapshot: Vec<(String, String)>,
}
