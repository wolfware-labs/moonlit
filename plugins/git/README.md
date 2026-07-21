# moonlit-plugin-git — Moonlit first-party git plugin

Five middlewares — `repo-context`, `latest-tag`, `commits`, `tag`, `push` —
built to a `wasm32-wasip2` component. **Excluded from the workspace** so the
engine build/CI never needs the wasm target; native unit tests run via
`cargo test --manifest-path plugins/git/Cargo.toml`.

## Regenerate the committed artifact

    cd plugins/git
    moonlit plugin build --release        # or: cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/git.wasm ../../engine/tests/fixtures/git.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/git.wasm`.
