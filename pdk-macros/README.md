# moonlit-pdk-macros

Procedural macro crate for [`moonlit-pdk`](https://crates.io/crates/moonlit-pdk).

Provides the `moonlit_plugin!` macro, which generates the WebAssembly component
entrypoints for a [Moonlit](https://github.com/wolfware-labs/moonlit) plugin from a list
of middlewares plus optional plugin config and shared state.

You normally depend on `moonlit-pdk` (which re-exports this macro) rather than on
this crate directly.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
