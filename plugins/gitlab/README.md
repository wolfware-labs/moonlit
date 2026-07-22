# moonlit-plugin-gitlab — Moonlit first-party gitlab plugin

Three middlewares — `related-items`, `create-release`, `write-variables` — built
to a `wasm32-wasip2` component over the GitLab REST API v4. **Excluded from the
workspace** so the engine build/CI never needs the wasm target; native unit tests
run via `cargo test --manifest-path plugins/gitlab/Cargo.toml`.

## Regenerate the committed artifact

    cd plugins/gitlab
    moonlit plugin build --release        # or: cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/gitlab.wasm ../../engine/tests/fixtures/gitlab.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/gitlab.wasm`.
