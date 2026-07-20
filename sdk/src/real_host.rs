//! The real host: implements `Host` against the generated wit-bindgen imports.
//! wasm-only — the native import stubs abort if called, so this whole module is
//! `cfg(target_arch = "wasm32")`. Binding-call shapes verified against wasm32-wasip2.

use crate::context::{Host, LogLevel};

/// The real host, backed by the wit-bindgen imports.
pub struct RealHost;

impl Host for RealHost {
    fn log(&self, level: LogLevel, message: &str) {
        use crate::bindings::moonlit::plugin::types::LogLevel as W;
        let w = match level {
            LogLevel::Debug => W::Debug,
            LogLevel::Info => W::Info,
            LogLevel::Warn => W::Warn,
            LogLevel::Error => W::Error,
        };
        crate::bindings::moonlit::plugin::host::log(w, message);
    }
    fn get_config(&self, path: &str) -> Option<String> {
        crate::bindings::moonlit::plugin::host::get_config(path)
    }
    fn report_progress(&self, message: &str) {
        crate::bindings::moonlit::plugin::host::report_progress(message);
    }
    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
    fn env_vars(&self) -> Vec<(String, String)> {
        std::env::vars().collect()
    }
}
