//! Verifies the `moonlit_plugin!`-generated metadata added in ABI 0.2.0:
//! `plugin-metadata.icon` (a compile-time-embedded `data:` URI) and each
//! `middleware-info.config-schema` (the JSON Schema of the middleware's `Config`).
//! A crate/test-binary may hold only ONE `moonlit_plugin!`, so the icon-present
//! and icon-absent cases live in separate test binaries (this one sets `icon:`;
//! `macro_dispatch.rs` omits it — see `describe_omits_icon_when_unset` there).

use moonlit_pdk::bindings::Guest;
use moonlit_pdk::prelude::*;

/// A documented input field so the emitted schema is non-trivial.
#[derive(serde::Deserialize, Default, schemars::JsonSchema)]
#[serde(default)]
struct TagInput {
    /// The tag prefix, e.g. `v`.
    prefix: String,
}

/// A documented output field so the emitted output-schema is non-trivial.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct TagOutput {
    /// The created tag, e.g. `v1.2.3`.
    tag: String,
}

#[derive(Default)]
struct Tag;
impl Middleware for Tag {
    const NAME: &'static str = "tag";
    const DESCRIPTION: &'static str = "creates a tag";
    type Input = TagInput;
    type Output = TagOutput;
    fn execute(&self, _ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output> {
        MiddlewareResult::ok(TagOutput {
            tag: format!("{}1.0.0", input.prefix),
        })
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
fn middleware_carries_input_and_output_schema_with_declared_fields() {
    let mws = <MoonlitComponent as Guest>::list_middlewares();
    assert_eq!(mws.len(), 1);

    let raw = mws[0]
        .input_schema
        .as_ref()
        .expect("input_schema must be emitted for every middleware");
    let schema: serde_json::Value =
        serde_json::from_str(raw).expect("input schema must be valid JSON");
    assert_eq!(
        schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
        "schema must declare the draft 2020-12 dialect"
    );
    assert!(
        schema.pointer("/properties/prefix").is_some(),
        "input schema must expose the `prefix` property; got {schema}"
    );

    let out_raw = mws[0]
        .output_schema
        .as_ref()
        .expect("output_schema must be emitted for every middleware");
    let out: serde_json::Value =
        serde_json::from_str(out_raw).expect("output schema must be valid JSON");
    assert!(
        out.pointer("/properties/tag").is_some(),
        "output schema must expose the `tag` property; got {out}"
    );
}
