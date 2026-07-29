# moonlit-plugin-sdk

SDK for writing [Moonlit](https://github.com/wolfware-labs/moonlit) release-pipeline
plugins as WebAssembly (`wasm32-wasip2`) components.

A Moonlit plugin is a `cdylib` that implements one or more middlewares over a `Context`
(HTTP, process exec, filesystem, environment, logging, and clock capabilities, each
gated by the host's deny-by-default sandbox). The `moonlit_plugin!` macro (from
`moonlit-plugin-sdk-macros`, re-exported here) wires your middlewares and optional
plugin config/state into the generated component entrypoints.

```rust
use moonlit_plugin_sdk::prelude::*;

#[derive(Default)]
struct Hello;

impl Middleware for Hello {
    const NAME: &'static str = "hello";
    const DESCRIPTION: &'static str = "say hello";
    type Config = ();

    fn execute(&self, ctx: &Context, _cfg: ()) -> MiddlewareResult {
        ctx.log_info("hello from a Moonlit plugin");
        MiddlewareResult::success()
    }
}

moonlit_plugin! { name: "hello", middlewares: [Hello] }
```

Build with `cargo build --target wasm32-wasip2 --release` (requires
`rustup target add wasm32-wasip2`).

## License

Elastic License 2.0 (`Elastic-2.0`).
