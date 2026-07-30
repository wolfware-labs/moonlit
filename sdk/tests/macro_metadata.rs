//! Verifies the `moonlit_plugin!`-generated metadata added in ABI 0.2.0:
//! `plugin-metadata.icon` (a compile-time-embedded `data:` URI) and each
//! `middleware-info.config-schema` (the JSON Schema of the middleware's `Config`).
//! A crate/test-binary may hold only ONE `moonlit_plugin!`, so the icon-present
//! and icon-absent cases live in separate test binaries (this one sets `icon:`;
//! `macro_dispatch.rs` omits it — see `describe_omits_icon_when_unset` there).

use moonlit_sdk::bindings::Guest;
use moonlit_sdk::prelude::*;

/// A documented config field so the emitted schema is non-trivial.
#[derive(serde::Deserialize, Default, schemars::JsonSchema)]
#[serde(default)]
struct TagCfg {
    /// The tag prefix, e.g. `v`.
    prefix: String,
}

#[derive(Default)]
struct Tag;
impl Middleware for Tag {
    const NAME: &'static str = "tag";
    const DESCRIPTION: &'static str = "creates a tag";
    type Config = TagCfg;
    fn execute(&self, _ctx: &Context, _cfg: Self::Config) -> MiddlewareResult {
        MiddlewareResult::success()
    }
}

moonlit_plugin! {
    name: "metadata-sample",
    icon: "tests/fixtures/icon.png",
    middlewares: [Tag],
}

#[test]
fn describe_embeds_icon_as_data_uri() {
    let meta = <MoonlitComponent as Guest>::describe();
    let icon = meta.icon.expect("icon must be present when `icon:` is set");
    assert!(
        icon.starts_with("data:image/png;base64,"),
        "icon must be a PNG data URI; got prefix {:?}",
        &icon[..icon.len().min(32)]
    );
    assert!(
        icon.len() > "data:image/png;base64,".len(),
        "data URI must carry bytes"
    );
}

#[test]
fn middleware_carries_config_schema_with_declared_field() {
    let mws = <MoonlitComponent as Guest>::list_middlewares();
    assert_eq!(mws.len(), 1);
    let raw = mws[0]
        .config_schema
        .as_ref()
        .expect("config_schema must be emitted for every middleware");
    let schema: serde_json::Value =
        serde_json::from_str(raw).expect("config schema must be valid JSON");
    assert_eq!(
        schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
        "schema must declare the draft 2020-12 dialect"
    );
    assert!(
        schema.pointer("/properties/prefix").is_some(),
        "schema must expose the `prefix` property; got {schema}"
    );
}
