//! Native test harness: drive a middleware against a recording mock host with
//! no wasm build. `run()` is added alongside the Middleware trait (Task 5).

use std::cell::RefCell;
use std::collections::HashMap;

use crate::context::{Host, LogLevel};

/// A recording, configurable host for native unit tests.
#[derive(Default)]
pub struct MockHost {
    logs: RefCell<Vec<(LogLevel, String)>>,
    progress: RefCell<Vec<String>>,
    config: HashMap<String, String>,
    env: HashMap<String, String>,
}

impl MockHost {
    pub fn new() -> Self {
        Self::default()
    }
    /// Serve `json_value` (json-value text, e.g. `"\"8080\""`) at `path`.
    #[must_use]
    pub fn with_config(mut self, path: &str, json_value: &str) -> Self {
        self.config.insert(path.to_string(), json_value.to_string());
        self
    }
    /// Serve `value` for env var `key`.
    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }
    pub fn logs(&self) -> Vec<(LogLevel, String)> {
        self.logs.borrow().clone()
    }
    pub fn progress(&self) -> Vec<String> {
        self.progress.borrow().clone()
    }
}

impl Host for MockHost {
    fn log(&self, level: LogLevel, message: &str) {
        self.logs.borrow_mut().push((level, message.to_string()));
    }
    fn get_config(&self, path: &str) -> Option<String> {
        self.config.get(path).cloned()
    }
    fn report_progress(&self, message: &str) {
        self.progress.borrow_mut().push(message.to_string());
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.env.get(name).cloned()
    }
    fn env_vars(&self) -> Vec<(String, String)> {
        self.env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

use crate::middleware::Middleware;
use crate::{Context, MiddlewareResult};

/// Run a middleware natively for unit tests.
pub fn run<M: Middleware>(mw: &M, ctx: &Context, cfg: M::Config) -> MiddlewareResult {
    mw.execute(ctx, cfg)
}
