//! Host bridge + Context. `Host` abstracts the wit-bindgen imports so the same
//! middleware runs under the real host (wasm) or a mock (native tests). The
//! real impl is cfg'd to wasm because the import stubs abort if called on
//! native targets.

use std::any::Any;

/// Log severity surfaced to the CLI renderer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// The capabilities `Context` needs from the environment.
pub trait Host {
    fn log(&self, level: LogLevel, message: &str);
    fn get_config(&self, path: &str) -> Option<String>;
    fn report_progress(&self, message: &str);
    /// Run a subprocess to completion, capturing output.
    fn process_run(
        &self,
        cmd: &crate::process::ProcessCommand,
    ) -> Result<crate::process::ProcessOutput, String>;
    /// Spawn a subprocess for streaming.
    fn process_spawn(
        &self,
        cmd: &crate::process::ProcessCommand,
    ) -> Result<Box<dyn crate::process::ChildHandle>, String>;
    /// Perform a blocking HTTP round-trip.
    fn http_send(
        &self,
        req: &crate::http::HttpRequestData,
    ) -> Result<crate::http::HttpResponseData, String>;
    /// Read an environment variable (routes to `wasi:cli/environment`, already
    /// permission-filtered by the engine). `None` if unset or filtered out.
    fn env_var(&self, name: &str) -> Option<String>;
    /// All visible environment variables.
    fn env_vars(&self) -> Vec<(String, String)>;
    /// Cryptographically-strong random bytes (routes to `wasi:random` on the
    /// real host; a deterministic mock in tests). Returns at least `n` bytes.
    fn random_bytes(&self, n: usize) -> Vec<u8>;
}

/// Execution context handed to every middleware.
pub struct Context<'a> {
    host: &'a dyn Host,
    working_dir: String,
    step_name: String,
    state: Option<&'a dyn Any>,
    plugin_config: Option<&'a dyn Any>,
}

impl<'a> Context<'a> {
    pub fn new(host: &'a dyn Host, working_dir: String, step_name: String) -> Self {
        Self {
            host,
            working_dir,
            step_name,
            state: None,
            plugin_config: None,
        }
    }
    #[must_use]
    pub fn with_state(mut self, state: &'a dyn Any) -> Self {
        self.state = Some(state);
        self
    }
    #[must_use]
    pub fn with_plugin_config(mut self, cfg: &'a dyn Any) -> Self {
        self.plugin_config = Some(cfg);
        self
    }

    pub fn log_debug(&self, msg: &str) {
        self.host.log(LogLevel::Debug, msg);
    }
    pub fn log_info(&self, msg: &str) {
        self.host.log(LogLevel::Info, msg);
    }
    pub fn log_warn(&self, msg: &str) {
        self.host.log(LogLevel::Warn, msg);
    }
    pub fn log_error(&self, msg: &str) {
        self.host.log(LogLevel::Error, msg);
    }
    pub fn progress(&self, msg: &str) {
        self.host.report_progress(msg);
    }

    pub fn working_dir(&self) -> &str {
        &self.working_dir
    }
    pub fn step_name(&self) -> &str {
        &self.step_name
    }

    /// Raw accumulated config at `path` (`:`-separated), parsed from json-value.
    pub fn get_config(&self, path: &str) -> Option<serde_json::Value> {
        let raw = self.host.get_config(path)?;
        serde_json::from_str(&raw).ok()
    }
    /// Config at `path`, coerced (§5.4) into `T`. `None` if absent or on error.
    pub fn get_config_as<T: serde::de::DeserializeOwned>(&self, path: &str) -> Option<T> {
        let raw = self.host.get_config(path)?;
        crate::config::from_json_value(&raw).ok()
    }

    /// Environment access.
    pub fn env(&self) -> crate::env::Env<'a> {
        crate::env::Env::new(self.host)
    }

    /// `n` random bytes from the host.
    pub fn random_bytes(&self, n: usize) -> Vec<u8> {
        self.host.random_bytes(n)
    }

    /// A random UUIDv4 string (`8-4-4-4-12` lowercase hex).
    pub fn uuid(&self) -> String {
        let mut b = self.host.random_bytes(16);
        b.resize(16, 0); // defensive: the contract is >= 16, but never index OOB
        b[6] = (b[6] & 0x0f) | 0x40; // version 4
        b[8] = (b[8] & 0x3f) | 0x80; // variant 1 (RFC 4122)
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }

    /// Subprocess builder for `program`.
    pub fn command(&self, program: impl Into<String>) -> crate::process::Command<'a> {
        crate::process::Command::new(self.host, program)
    }

    /// Blocking HTTP client.
    pub fn http(&self) -> crate::http::Http<'a> {
        crate::http::Http::new(self.host)
    }

    /// The plugin's shared state. Panics if the plugin declared no `state:`.
    pub fn state<T: 'static>(&self) -> &T {
        self.state
            .expect("this plugin declared no `state:` in moonlit_plugin!")
            .downcast_ref::<T>()
            .expect("state type mismatch")
    }
    /// The typed plugin-level config. Panics if the plugin declared no `config:`.
    pub fn plugin_config<T: 'static>(&self) -> &T {
        self.plugin_config
            .expect("this plugin declared no `config:` in moonlit_plugin!")
            .downcast_ref::<T>()
            .expect("plugin config type mismatch")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockHost;

    #[test]
    fn log_and_progress_route_to_host() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "compile".into());
        ctx.log_info("hi");
        ctx.log_error("bad");
        ctx.progress("50%");
        assert_eq!(ctx.working_dir(), "/w");
        assert_eq!(ctx.step_name(), "compile");
        assert_eq!(
            host.logs(),
            vec![
                (LogLevel::Info, "hi".to_string()),
                (LogLevel::Error, "bad".to_string()),
            ]
        );
        assert_eq!(host.progress(), vec!["50%".to_string()]);
    }

    #[test]
    fn get_config_parses_and_coerces() {
        let host = MockHost::new().with_config("plugin:port", "\"8080\"");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert_eq!(
            ctx.get_config("plugin:port"),
            Some(serde_json::json!("8080"))
        );
        assert_eq!(ctx.get_config_as::<i64>("plugin:port"), Some(8080));
        assert_eq!(ctx.get_config_as::<i64>("missing"), None);
    }

    #[test]
    fn state_downcasts() {
        struct S {
            n: u32,
        }
        let host = MockHost::new();
        let s = S { n: 7 };
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_state(&s);
        assert_eq!(ctx.state::<S>().n, 7);
    }

    #[test]
    #[should_panic(expected = "this plugin declared no `state:` in moonlit_plugin!")]
    fn state_panics_when_plugin_declared_none() {
        struct S;
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        ctx.state::<S>();
    }

    #[test]
    #[should_panic(expected = "this plugin declared no `config:` in moonlit_plugin!")]
    fn plugin_config_panics_when_plugin_declared_none() {
        struct C;
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        ctx.plugin_config::<C>();
    }

    #[test]
    fn uuid_is_v4_formatted_and_deterministic_under_mock() {
        use crate::testing::MockHost;
        let host = MockHost::new().with_random(&[0xab]); // every byte 0xab
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let u = ctx.uuid();
        // 0xab: b[6]=(0x0b|0x40)=0x4b (version 4); b[8]=(0x2b|0x80)=0xab (variant 1)
        assert_eq!(u, "abababab-abab-4bab-abab-abababababab");
        assert_eq!(u, ctx.uuid(), "deterministic under a fixed mock seed");
    }

    #[test]
    fn random_bytes_passthrough_returns_requested_length() {
        use crate::testing::MockHost;
        let host = MockHost::new().with_random(&[1, 2, 3]);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert_eq!(ctx.random_bytes(5), vec![1, 2, 3, 1, 2]); // cycles the seed
    }
}
