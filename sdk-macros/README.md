# moonlit-sdk-macros

Procedural macro crate for [`moonlit-sdk`](https://crates.io/crates/moonlit-sdk).

Provides the `moonlit_plugin!` macro, which generates the WebAssembly component
entrypoints for a [Moonlit](https://github.com/wolfware-labs/moonlit) plugin from a list
of middlewares plus optional plugin config and shared state.

You normally depend on `moonlit-sdk` (which re-exports this macro) rather than on
this crate directly.

## License

Elastic License 2.0 (`Elastic-2.0`).
