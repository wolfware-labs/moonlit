# moonlit-plugin-github — Moonlit first-party github plugin

Three middlewares — `related-items`, `create-release`, `write-variables` — built
to a `wasm32-wasip2` component over the GitHub REST API. **Excluded from the
workspace** so the engine build/CI never needs the wasm target; native unit tests
run via `cargo test --manifest-path plugins/github/Cargo.toml`.

## Regenerate the committed artifact

    cd plugins/github
    moonlit plugin build --release        # or: cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/github.wasm ../../engine/tests/fixtures/github.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/github.wasm`.
