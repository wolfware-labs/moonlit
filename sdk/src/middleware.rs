//! The middleware authoring trait. A middleware is a `Default` unit struct with
//! a name, a description, a typed config, and an `execute`.

use crate::{Context, MiddlewareResult};

pub trait Middleware: Default {
    /// The `run:` reference name (e.g. `latest-tag`), unique within the plugin.
    const NAME: &'static str;
    /// Shown by `plugin inspect` / `list-middlewares`.
    const DESCRIPTION: &'static str = "";
    /// The step config type; `Default` so an absent block still binds.
    type Config: serde::de::DeserializeOwned + Default;
    fn execute(&self, ctx: &Context, cfg: Self::Config) -> MiddlewareResult;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::testing::{run, MockHost};

    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    struct GreetCfg {
        name: String,
        times: i64,
    }

    #[derive(Default)]
    struct Greet;
    impl Middleware for Greet {
        const NAME: &'static str = "greet";
        const DESCRIPTION: &'static str = "greets";
        type Config = GreetCfg;
        fn execute(&self, ctx: &Context, cfg: Self::Config) -> MiddlewareResult {
            ctx.log_info(&format!("hi {}", cfg.name));
            MiddlewareResult::success_with(|o| {
                o.set("count", cfg.times);
            })
        }
    }

    #[test]
    fn run_drives_a_middleware_natively() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        // times arrives as a coerced string, proving the caller-side coercion path.
        let cfg: GreetCfg =
            crate::config::from_json_value(r#"{"name":"ada","times":"3"}"#).unwrap();
        let w = run(&Greet, &ctx, cfg).into_wit();
        assert!(w.successful);
        assert_eq!(host.logs()[0].1, "hi ada");
        assert_eq!(w.output.iter().find(|(k, _)| k == "count").unwrap().1, "3");
    }
}
