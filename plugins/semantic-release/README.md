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

## 2.0 vs 1.x divergences

- `prereleaseMappings` glob keys resolve by exact-key-then-alphabetical-glob
  precedence: engine config maps are unordered, so 1.x's declaration-order
  glob matching is not available.
- Rule-config enum values (`VersionBumpType`) are case-sensitive PascalCase
  (e.g. `Major`, `Minor`, `Patch`, `None`), unlike 1.x's case-insensitive binder.
- Custom `ChangelogRule` entries must specify `icon`, `section`, and `summary`
  explicitly — there are no silent per-property defaults; a rule missing one
  of these fails to deserialize instead of falling back to a default value.
- Malformed config values (e.g. an invalid semver in a prerelease mapping, or
  a non-ASCII sha) surface as a run failure rather than being silently
  coerced or skipped.
