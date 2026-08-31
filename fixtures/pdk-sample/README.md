# pdk-sample — Moonlit SDK integration fixture

A plugin written on `moonlit-pdk`, built to a `wasm32-wasip2` component
and used by `engine/tests/pdk_sample.rs`. **Excluded from the workspace** so the
engine build/CI never needs the wasm target.

## Regenerate the committed artifact

    cd fixtures/pdk-sample
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/pdk_sample.wasm ../../engine/tests/fixtures/pdk_sample.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with `wasm-tools validate`
and `wasm-tools print <wasm> | head -1` (must start with `(component`).
