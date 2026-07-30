# test-plugin — Moonlit host test fixture

A wit-bindgen guest implementing `world plugin` (`moonlit:plugin@0.2.0`), built to a
`wasm32-wasip2` component and used by the engine's `host` integration tests. It is
**excluded from the workspace** so the engine build/CI never needs the wasm target.

## Regenerate the committed artifact

    cd fixtures/test-plugin
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/test_plugin.wasm ../../engine/tests/fixtures/test_plugin.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with `wasm-tools validate` and
`wasm-tools print <wasm> | head -1` (must start with `(component`).
