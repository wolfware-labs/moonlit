//! Minimal plugin built on moonlit-plugin-sdk, used by the engine integration
//! test to prove the SDK produces a real, runnable component. SDK-core only:
//! logging, progress, typed config coercion, get-config, outputs, shared state.

use moonlit_plugin_sdk::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct EchoConfig {
    times: i64,
    label: String,
}

#[derive(Default)]
struct Echo;

impl Middleware for Echo {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "echoes config and reads plugin:name";
    type Config = EchoConfig;
    fn execute(&self, ctx: &Context, cfg: Self::Config) -> MiddlewareResult {
        ctx.log_info(&format!("echo in {}", ctx.working_dir()));
        ctx.progress("working");
        let seen = ctx
            .get_config("plugin:name")
            .unwrap_or(serde_json::Value::Null);
        MiddlewareResult::success_with(|o| {
            o.set("times", cfg.times);      // coerced from a string by the SDK
            o.set("label", cfg.label);
            o.set("step", ctx.step_name());
            o.set("plugin_name", seen);
        })
        .with_warning("sample warning")
    }
}

#[derive(Default)]
struct Fail;
impl Middleware for Fail {
    const NAME: &'static str = "fail";
    const DESCRIPTION: &'static str = "always fails";
    type Config = EchoConfig;
    fn execute(&self, _ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        MiddlewareResult::failure("intentional failure")
    }
}

moonlit_plugin! {
    name: "sdk-sample",
    middlewares: [Echo, Fail],
}
