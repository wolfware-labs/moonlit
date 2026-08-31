//! Environment access. Reads route through the host, which serves
//! `wasi:cli/environment` — already permission-filtered by the engine
//! (`filter_env`). Funnelling through `Host` keeps env mockable in native tests.

use crate::context::Host;

/// Environment handle, created via `ctx.env()`.
pub struct Env<'a> {
    host: &'a dyn Host,
}

impl<'a> Env<'a> {
    pub(crate) fn new(host: &'a dyn Host) -> Self {
        Self { host }
    }
    /// The value of `name`, or `None` if unset (or filtered out by permissions).
    pub fn var(&self, name: &str) -> Option<String> {
        self.host.env_var(name)
    }
    /// `name`'s value, or `default` if unset.
    pub fn var_or(&self, name: &str, default: impl Into<String>) -> String {
        self.host.env_var(name).unwrap_or_else(|| default.into())
    }
    /// All visible environment variables.
    pub fn vars(&self) -> Vec<(String, String)> {
        self.host.env_vars()
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::MockHost;
    use crate::Context;

    #[test]
    fn env_reads_var_vars_and_default() {
        let host = MockHost::new().with_env("TOKEN", "abc");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert_eq!(ctx.env().var("TOKEN"), Some("abc".to_string()));
        assert_eq!(ctx.env().var("MISSING"), None);
        assert_eq!(ctx.env().var_or("MISSING", "def"), "def");
        assert_eq!(
            ctx.env().vars(),
            vec![("TOKEN".to_string(), "abc".to_string())]
        );
    }
}
