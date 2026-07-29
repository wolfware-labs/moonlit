# moonlit-sdk

SDK for writing [Moonlit](https://github.com/wolfware-labs/moonlit) release-pipeline
plugins as WebAssembly (`wasm32-wasip2`) components.

A Moonlit plugin is a `cdylib` that implements one or more middlewares over a `Context`
(HTTP, process exec, filesystem, environment, logging, and clock capabilities, each
gated by the host's deny-by-default sandbox). The `moonlit_plugin!` macro (from
`moonlit-sdk-macros`, re-exported here) wires your middlewares and optional
plugin config/state into the generated component entrypoints.

```rust
use moonlit_sdk::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
struct HelloConfig {
    name: String,
}

#[derive(Default)]
struct Hello;

impl Middleware for Hello {
    const NAME: &'static str = "hello";
    const DESCRIPTION: &'static str = "say hello";
    type Config = HelloConfig;

    fn execute(&self, ctx: &Context, cfg: HelloConfig) -> MiddlewareResult {
        let name = if cfg.name.is_empty() { "world" } else { &cfg.name };
        ctx.log_info(&format!("hello, {name}"));
        MiddlewareResult::success()
    }
}

moonlit_plugin! { name: "hello", middlewares: [Hello] }
```

Each middleware's `Config` is deserialized from its step config (a `#[serde(default)]`
struct is the idiomatic "all fields optional" choice).

Build with `cargo build --target wasm32-wasip2 --release` (requires
`rustup target add wasm32-wasip2`).

## License

Elastic License 2.0 (`Elastic-2.0`).
