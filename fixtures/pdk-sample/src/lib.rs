use moonlit_pdk::prelude::*;

#[derive(Deserialize, Default, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct EchoInput {
    times: i64,
    label: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct EchoOutput {
    times: i64,
    label: String,
    step: String,
    plugin_name: serde_json::Value,
}

#[derive(Default)]
struct Echo;

impl Middleware for Echo {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "echoes config and reads plugin:name";
    type Input = EchoInput;
    type Output = EchoOutput;
    fn execute(&self, ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output> {
        ctx.log_info(&format!("echo in {}", ctx.working_dir()));
        ctx.progress("working");
        let seen = ctx
            .get_config("plugin:name")
            .unwrap_or(serde_json::Value::Null);
        MiddlewareResult::ok(EchoOutput {
            times: input.times,
            label: input.label,
            step: ctx.step_name().to_string(),
            plugin_name: seen,
        })
        .with_warning("sample warning")
    }
}

#[derive(Default)]
struct Fail;
impl Middleware for Fail {
    const NAME: &'static str = "fail";
    const DESCRIPTION: &'static str = "always fails";
    type Input = EchoInput;
    type Output = NoOutput;
    fn execute(&self, _ctx: &Context, _input: Self::Input) -> MiddlewareResult<Self::Output> {
        MiddlewareResult::failure("intentional failure")
    }
}

#[derive(Default)]
struct RunEcho;

#[derive(Serialize, schemars::JsonSchema)]
struct RunEchoOutput {
    exit_code: i32,
    stdout: String,
}

impl Middleware for RunEcho {
    const NAME: &'static str = "run-echo";
    const DESCRIPTION: &'static str = "run `echo hello` and capture output";
    type Input = NoInput;
    type Output = RunEchoOutput;
    fn execute(&self, ctx: &Context, _input: Self::Input) -> MiddlewareResult<Self::Output> {
        match ctx.command("echo").arg("hello").run() {
            Ok(out) => MiddlewareResult::ok(RunEchoOutput {
                exit_code: out.exit_code,
                stdout: out.stdout(),
            }),
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[derive(Default)]
struct SpawnEcho;

#[derive(Serialize, schemars::JsonSchema)]
struct SpawnEchoOutput {
    exit_code: i32,
    lines: String,
}

impl Middleware for SpawnEcho {
    const NAME: &'static str = "spawn-echo";
    const DESCRIPTION: &'static str = "spawn `echo hello` and stream lines";
    type Input = NoInput;
    type Output = SpawnEchoOutput;
    fn execute(&self, ctx: &Context, _input: Self::Input) -> MiddlewareResult<Self::Output> {
        match ctx.command("echo").arg("hello").spawn() {
            Ok(mut child) => {
                let mut lines = Vec::new();
                while let Some(c) = child.next_line() {
                    lines.push(c.text);
                }
                let code = child.wait();
                MiddlewareResult::ok(SpawnEchoOutput {
                    exit_code: code,
                    lines: lines.join(","),
                })
            }
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[derive(Deserialize, Default, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct HttpGetInput {
    scheme: String,
    authority: String,
    path: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct HttpGetOutput {
    status: u16,
    body: String,
}

#[derive(Default)]
struct HttpGet;
impl Middleware for HttpGet {
    const NAME: &'static str = "http-get";
    const DESCRIPTION: &'static str = "GET a URL and return status + body";
    type Input = HttpGetInput;
    type Output = HttpGetOutput;
    fn execute(&self, ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output> {
        let url = format!("{}://{}{}", input.scheme, input.authority, input.path);
        match ctx.http().get(url).send() {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                MiddlewareResult::ok(HttpGetOutput { status, body })
            }
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
struct ReadEnvOutput {
    #[serde(rename = "SAMPLE_ENV")]
    sample_env: String,
}

#[derive(Default)]
struct ReadEnv;
impl Middleware for ReadEnv {
    const NAME: &'static str = "read-env";
    const DESCRIPTION: &'static str = "read SAMPLE_ENV from the environment";
    type Input = NoInput;
    type Output = ReadEnvOutput;
    fn execute(&self, ctx: &Context, _input: Self::Input) -> MiddlewareResult<Self::Output> {
        let val = ctx.env().var("SAMPLE_ENV").unwrap_or_default();
        MiddlewareResult::ok(ReadEnvOutput { sample_env: val })
    }
}

moonlit_plugin! {
    name: "pdk-sample",
    icon: "icon.png",
    middlewares: [Echo, Fail, RunEcho, SpawnEcho, HttpGet, ReadEnv],
}
