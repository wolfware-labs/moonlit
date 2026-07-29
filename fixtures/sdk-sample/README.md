# sdk-sample — Moonlit SDK integration fixture

A plugin written on `moonlit-sdk`, built to a `wasm32-wasip2` component
and used by `engine/tests/sdk_sample.rs`. **Excluded from the workspace** so the
engine build/CI never needs the wasm target.

## Regenerate the committed artifact

    cd fixtures/sdk-sample
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/sdk_sample.wasm ../../engine/tests/fixtures/sdk_sample.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with `wasm-tools validate`
and `wasm-tools print <wasm> | head -1` (must start with `(component`).
