# moonlit-pdk

SDK for writing [Moonlit](https://github.com/wolfware-labs/moonlit) release-pipeline
plugins as WebAssembly (`wasm32-wasip2`) components.

A Moonlit plugin is a `cdylib` that implements one or more middlewares over a `Context`
(HTTP, process exec, filesystem, environment, logging, and clock capabilities, each
gated by the host's deny-by-default sandbox). The `moonlit_plugin!` macro (from
`moonlit-pdk-macros`, re-exported here) wires your middlewares and optional
plugin config/state into the generated component entrypoints.

```rust
use moonlit_pdk::prelude::*;

/// Input for the `greet` middleware, bound from the step's config block.
/// `JsonSchema` is what lets `moonlit plugin inspect` and the registry
/// document these fields.
#[derive(Deserialize, Default, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct GreetInput {
    /// Who to greet; defaults to "world" when empty.
    name: String,
}

/// Output published for downstream steps to read as
/// `steps.<step>.outputs.greeting`.
#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GreetOutput {
    /// The greeting message, e.g. `hello, world`.
    greeting: String,
}

#[derive(Default)]
struct Greet;

impl Middleware for Greet {
    const NAME: &'static str = "greet";
    const DESCRIPTION: &'static str = "logs and returns a greeting";
    type Input = GreetInput;
    type Output = GreetOutput;

    fn execute(&self, ctx: &Context, input: Self::Input) -> MiddlewareResult<Self::Output> {
        let who = if input.name.is_empty() {
            "world".to_string()
        } else {
            input.name
        };
        ctx.log_info(&format!("greeting {who}"));
        MiddlewareResult::ok(GreetOutput {
            greeting: format!("hello, {who}"),
        })
    }
}

moonlit_plugin! { name: "greet-plugin", middlewares: [Greet] }
```

Each middleware declares two types. `Input` is deserialized from the step's config
block (a `#[serde(default)]` struct is the idiomatic "all fields optional" choice);
`Output` is serialized and published for later steps to read as
`steps.<step>.outputs.<field>`. Use `NoInput` or `NoOutput` when a middleware takes
no configuration or publishes nothing. Both types derive `schemars::JsonSchema`, and
the macro emits those schemas into the component so `moonlit plugin inspect` and the
registry can document the plugin without running it.

Alongside `moonlit-pdk`, a plugin crate needs `serde` (with `derive`),
`serde_json`, and `schemars = "1"`. `moonlit plugin new` scaffolds all of this —
the example above is what it generates.

Build with `cargo build --target wasm32-wasip2 --release` (requires
`rustup target add wasm32-wasip2`).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
