//! Exercises the `moonlit_plugin!`-generated `Guest` impl natively (host target).
//! The generated code uses `__MoonlitUnavailableHost` off-wasm, so we assert on
//! init metadata, the middleware list, and unknown-middleware dispatch — paths
//! that do not depend on host calls.

use moonlit_sdk::bindings::Guest;
use moonlit_sdk::prelude::*;

#[derive(serde::Deserialize, Default, schemars::JsonSchema)]
#[serde(default)]
struct NoCfg {}

#[derive(Default)]
struct Alpha;
impl Middleware for Alpha {
    const NAME: &'static str = "alpha";
    const DESCRIPTION: &'static str = "first";
    type Config = NoCfg;
    fn execute(&self, _ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        MiddlewareResult::success()
    }
}

moonlit_plugin! {
    name: "sample",
    middlewares: [Alpha],
}

#[test]
fn init_reports_name_and_version() {
    let meta = <MoonlitComponent as Guest>::init("{}".to_string()).unwrap();
    assert_eq!(meta.name, "sample");
    assert_eq!(meta.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn lists_declared_middlewares() {
    let mws = <MoonlitComponent as Guest>::list_middlewares();
    assert_eq!(mws.len(), 1);
    assert_eq!(mws[0].name, "alpha");
    assert_eq!(mws[0].description, "first");
}

#[test]
fn describe_omits_icon_when_unset() {
    // This plugin declares no `icon:`, so the ABI-0.2.0 icon field is None.
    assert!(<MoonlitComponent as Guest>::describe().icon.is_none());
}
