# moonlit-plugin-semantic-release — Moonlit first-party semantic-release plugin

Three middlewares — `analyze`, `calculate-version`, `generate-changelog` — built to a
`wasm32-wasip2` component. Pure-Rust and **fully offline** (no network/exec/filesystem
permissions). **Excluded from the workspace** so the engine build/CI never needs the
wasm target; native unit tests run via
`cargo test --manifest-path plugins/semantic-release/Cargo.toml`.

## Regenerate the committed artifact

    cd plugins/semantic-release
    moonlit plugin build --release        # or: cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/semantic_release.wasm ../../engine/tests/fixtures/semantic-release.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/semantic-release.wasm`.
