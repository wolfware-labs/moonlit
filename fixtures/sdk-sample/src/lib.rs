//! Minimal plugin built on moonlit-sdk, used by the engine integration
//! tests to prove the SDK produces a real, runnable component. Exercises SDK
//! core (Echo/Fail) plus the utility modules (process/http/env).

use moonlit_sdk::prelude::*;

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
            o.set("times", cfg.times);
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

/// A config type for middlewares that take no parameters.
#[derive(Deserialize, Default)]
#[serde(default)]
struct NoConfig {}

#[derive(Default)]
struct RunEcho;
impl Middleware for RunEcho {
    const NAME: &'static str = "run-echo";
    const DESCRIPTION: &'static str = "run `echo hello` and capture output";
    type Config = NoConfig;
    fn execute(&self, ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        match ctx.command("echo").arg("hello").run() {
            Ok(out) => MiddlewareResult::success_with(|o| {
                o.set("exit_code", out.exit_code);
                o.set("stdout", out.stdout());
            }),
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[derive(Default)]
struct SpawnEcho;
impl Middleware for SpawnEcho {
    const NAME: &'static str = "spawn-echo";
    const DESCRIPTION: &'static str = "spawn `echo hello` and stream lines";
    type Config = NoConfig;
    fn execute(&self, ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        match ctx.command("echo").arg("hello").spawn() {
            Ok(mut child) => {
                let mut lines = Vec::new();
                while let Some(c) = child.next_line() {
                    lines.push(c.text);
                }
                let code = child.wait();
                MiddlewareResult::success_with(|o| {
                    o.set("exit_code", code);
                    o.set("lines", lines.join(","));
                })
            }
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct HttpGetConfig {
    scheme: String,
    authority: String,
    path: String,
}

#[derive(Default)]
struct HttpGet;
impl Middleware for HttpGet {
    const NAME: &'static str = "http-get";
    const DESCRIPTION: &'static str = "GET a URL and return status + body";
    type Config = HttpGetConfig;
    fn execute(&self, ctx: &Context, cfg: Self::Config) -> MiddlewareResult {
        let url = format!("{}://{}{}", cfg.scheme, cfg.authority, cfg.path);
        match ctx.http().get(url).send() {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                MiddlewareResult::success_with(|o| {
                    o.set("status", status);
                    o.set("body", body);
                })
            }
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[derive(Default)]
struct ReadEnv;
impl Middleware for ReadEnv {
    const NAME: &'static str = "read-env";
    const DESCRIPTION: &'static str = "read SAMPLE_ENV from the environment";
    type Config = NoConfig;
    fn execute(&self, ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        let val = ctx.env().var("SAMPLE_ENV").unwrap_or_default();
        MiddlewareResult::success_with(|o| {
            o.set("SAMPLE_ENV", val);
        })
    }
}

moonlit_plugin! {
    name: "sdk-sample",
    middlewares: [Echo, Fail, RunEcho, SpawnEcho, HttpGet, ReadEnv],
}
