//! Exercises the `moonlit_plugin!` `state:` and `config:` expansion branches,
//! which have no other coverage: the `__MOONLIT_STATE` / `__MOONLIT_PLUGIN_CONFIG`
//! `OnceLock` statics, the init-time plugin-config decode (incl. the `Err` ->
//! exit-3 path), and `ctx.with_state` / `ctx.with_plugin_config` attachment.
//!
//! A crate/test-binary may contain only ONE `moonlit_plugin!` invocation (it
//! emits `struct MoonlitComponent` + `export!`), so this lives in its own test
//! binary, separate from `macro_dispatch.rs`.
//!
//! Both assertions run inside a single `#[test]` because the generated
//! `OnceLock` statics are process-global: `OnceLock::set` only succeeds once
//! per process, so splitting into multiple `#[test]`s (which may run in the
//! same process) would make `init` order-dependent and flaky.

use moonlit_sdk::bindings::{Guest, ReleaseContext};
use moonlit_sdk::prelude::*;

#[derive(Default)]
struct Counter {
    hits: std::sync::Mutex<u32>,
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct PluginCfg {
    token: String,
}

impl PluginConfig for PluginCfg {
    fn validate(&self) -> Result<(), String> {
        if self.token.trim().is_empty() {
            return Err("token is required.".to_string());
        }
        Ok(())
    }
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct NoCfg {}

#[derive(Default)]
struct Beta;
impl Middleware for Beta {
    const NAME: &'static str = "beta";
    const DESCRIPTION: &'static str = "second";
    type Config = NoCfg;
    fn execute(&self, ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        let mut hits = ctx.state::<Counter>().hits.lock().unwrap();
        *hits += 1;
        let hits = *hits;
        let token = ctx.plugin_config::<PluginCfg>().token.clone();
        MiddlewareResult::success_with(|o| {
            o.set("token", token);
            o.set("hits", hits);
        })
    }
}

moonlit_plugin! {
    name: "stateful",
    config: PluginCfg,
    middlewares: [Beta],
    state: Counter,
}

#[test]
fn state_and_config_flow_through_generated_code() {
    // Non-object plugin-config: `from_json_value::<PluginCfg>` fails to decode
    // (a bare JSON string cannot deserialize into a struct), so `init` returns
    // `Err` (engine exit 3). The `__MOONLIT_PLUGIN_CONFIG.set` is never reached.
    #[allow(clippy::single_match)]
    match <MoonlitComponent as Guest>::init("\"not-an-object\"".to_string()) {
        Ok(_) => panic!("expected Err for non-object plugin config"),
        Err(_) => (),
    }

    // Blank token: parses, then `validate()` fails — verbatim message, no `.set`.
    match <MoonlitComponent as Guest>::init(r#"{"token":""}"#.to_string()) {
        Ok(_) => panic!("blank token must fail init"),
        Err(e) => assert_eq!(e, "token is required."),
    }

    // Valid plugin-config: init succeeds and stores it (the earlier failed
    // attempts returned before calling `.set`, so this `.set` is the first and
    // only successful one for the process-global OnceLock).
    let meta = <MoonlitComponent as Guest>::init(r#"{"token":"abc"}"#.to_string())
        .expect("valid plugin config must init successfully");
    assert_eq!(meta.name, "stateful");

    // Execute the sole middleware: `state` lazily default-inits `Counter` and
    // increments it; `plugin_config` reads the value stored above.
    let result = <MoonlitComponent as Guest>::execute(
        "beta".to_string(),
        ReleaseContext {
            working_directory: "/w".to_string(),
            step_name: "s".to_string(),
        },
        "{}".to_string(),
    );

    assert!(result.successful);
    let output: std::collections::HashMap<_, _> = result.output.into_iter().collect();
    assert_eq!(output["token"], "\"abc\"");
    assert_eq!(output["hits"], "1");
}

#[test]
fn describe_returns_metadata_without_validating_config() {
    // `describe` is the config-free metadata path (`moonlit plugin inspect` uses
    // it): it must return name/version even though this plugin's `validate()`
    // rejects the blank config `init` would be handed. It never touches the
    // plugin-config `OnceLock`, so it is independent of init ordering.
    let meta = <MoonlitComponent as Guest>::describe();
    assert_eq!(meta.name, "stateful");
    assert_eq!(meta.version, env!("CARGO_PKG_VERSION"));
}
