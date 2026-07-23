//! Native test harness: drive a middleware against a recording mock host with
//! no wasm build. `run()` is added alongside the Middleware trait (Task 5).

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use crate::context::{Host, LogLevel};
use crate::http::{HttpRequestData, HttpResponseData};
use crate::process::{ChildHandle, OutputChunk, ProcessCommand, ProcessOutput};

/// A recording, configurable host for native unit tests.
#[derive(Default)]
pub struct MockHost {
    logs: RefCell<Vec<(LogLevel, String)>>,
    progress: RefCell<Vec<String>>,
    config: HashMap<String, String>,
    env: HashMap<String, String>,
    process_results: RefCell<VecDeque<Result<ProcessOutput, String>>>,
    recorded_commands: RefCell<Vec<ProcessCommand>>,
    http_responses: RefCell<VecDeque<Result<HttpResponseData, String>>>,
    recorded_requests: RefCell<Vec<HttpRequestData>>,
    random: Vec<u8>,
    clock: Vec<u64>,
    cursor: RefCell<usize>,
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
    /// Seed the deterministic random source (cycled to fill any request).
    #[must_use]
    pub fn with_random(mut self, bytes: &[u8]) -> Self {
        self.random = bytes.to_vec();
        self
    }
    /// Script successive monotonic readings (nanoseconds), returned in order;
    /// the last value repeats once the script is exhausted (extra reads never
    /// panic). Empty script → always 0.
    #[must_use]
    pub fn with_clock(mut self, nanos: &[u64]) -> Self {
        self.clock = nanos.to_vec();
        self
    }
    pub fn logs(&self) -> Vec<(LogLevel, String)> {
        self.logs.borrow().clone()
    }
    pub fn progress(&self) -> Vec<String> {
        self.progress.borrow().clone()
    }
    /// Enqueue a successful process result (also used as a `spawn` line script).
    #[must_use]
    pub fn with_process_result(self, exit_code: i32, chunks: Vec<OutputChunk>) -> Self {
        self.process_results
            .borrow_mut()
            .push_back(Ok(ProcessOutput { exit_code, chunks }));
        self
    }
    /// Enqueue a spawn/run failure.
    #[must_use]
    pub fn with_process_error(self, msg: &str) -> Self {
        self.process_results
            .borrow_mut()
            .push_back(Err(msg.to_string()));
        self
    }
    /// Commands passed to `process_run`/`process_spawn`, in order.
    pub fn recorded_commands(&self) -> Vec<ProcessCommand> {
        self.recorded_commands.borrow().clone()
    }
    /// Enqueue a successful response with no headers.
    #[must_use]
    pub fn with_http_response(self, status: u16, body: &[u8]) -> Self {
        self.http_responses
            .borrow_mut()
            .push_back(Ok(HttpResponseData {
                status,
                headers: Vec::new(),
                body: body.to_vec(),
            }));
        self
    }
    /// Enqueue a successful response with explicit headers.
    #[must_use]
    pub fn with_http_response_headers(
        self,
        status: u16,
        headers: Vec<(String, String)>,
        body: &[u8],
    ) -> Self {
        self.http_responses
            .borrow_mut()
            .push_back(Ok(HttpResponseData {
                status,
                headers,
                body: body.to_vec(),
            }));
        self
    }
    /// Enqueue a transport failure.
    #[must_use]
    pub fn with_http_error(self, msg: &str) -> Self {
        self.http_responses
            .borrow_mut()
            .push_back(Err(msg.to_string()));
        self
    }
    /// Requests passed to `http_send`, in order.
    pub fn recorded_requests(&self) -> Vec<HttpRequestData> {
        self.recorded_requests.borrow().clone()
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
    fn process_run(&self, cmd: &ProcessCommand) -> Result<ProcessOutput, String> {
        self.recorded_commands.borrow_mut().push(cmd.clone());
        self.process_results
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Err("MockHost: no process result configured".to_string()))
    }
    fn process_spawn(&self, cmd: &ProcessCommand) -> Result<Box<dyn ChildHandle>, String> {
        self.recorded_commands.borrow_mut().push(cmd.clone());
        match self.process_results.borrow_mut().pop_front() {
            Some(Ok(out)) => Ok(Box::new(MockChild {
                lines: out.chunks.into(),
                exit_code: out.exit_code,
            })),
            Some(Err(e)) => Err(e),
            None => Err("MockHost: no process result configured".to_string()),
        }
    }
    fn http_send(&self, req: &HttpRequestData) -> Result<HttpResponseData, String> {
        self.recorded_requests.borrow_mut().push(req.clone());
        self.http_responses
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Err("MockHost: no http response configured".to_string()))
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
    fn random_bytes(&self, n: usize) -> Vec<u8> {
        if self.random.is_empty() {
            (0..n).map(|i| i as u8).collect()
        } else {
            (0..n).map(|i| self.random[i % self.random.len()]).collect()
        }
    }
    fn monotonic_nanos(&self) -> u64 {
        if self.clock.is_empty() {
            return 0;
        }
        let mut c = self.cursor.borrow_mut();
        let v = self.clock[(*c).min(self.clock.len() - 1)];
        *c += 1;
        v
    }
}

use crate::middleware::Middleware;
use crate::{Context, MiddlewareResult};

/// Run a middleware natively for unit tests.
pub fn run<M: Middleware>(mw: &M, ctx: &Context, cfg: M::Config) -> MiddlewareResult {
    mw.execute(ctx, cfg)
}

struct MockChild {
    lines: VecDeque<OutputChunk>,
    exit_code: i32,
}

impl ChildHandle for MockChild {
    fn next_line(&mut self) -> Option<OutputChunk> {
        self.lines.pop_front()
    }
    fn wait(&mut self) -> i32 {
        self.exit_code
    }
    fn kill(&mut self) {}
}

#[cfg(test)]
mod random_tests {
    use super::MockHost;
    use crate::context::Host;

    #[test]
    fn default_random_is_deterministic_and_sized() {
        let h = MockHost::new();
        assert_eq!(h.random_bytes(4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn with_random_cycles_the_seed() {
        let h = MockHost::new().with_random(&[9, 8]);
        assert_eq!(h.random_bytes(3), vec![9, 8, 9]);
    }
}
