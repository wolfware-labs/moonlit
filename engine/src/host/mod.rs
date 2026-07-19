//! wasmtime component host for `moonlit:plugin@2.0.0` (Phase 5): instantiate one
//! resolved plugin and call its exports, with the full host ABI + permission
//! enforcement. bindgen wiring and the `PluginInstance` seam arrive in later tasks.

mod convert;

pub use convert::{HostError, json_str_to_value, value_to_json};
pub use convert::{LogLevel, MiddlewareInfo, MiddlewareResult, PluginMetadata, ReleaseContext};
