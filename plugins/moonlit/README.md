# moonlit-plugin-moonlit

First-party Moonlit plugin. Runs nested Moonlit release files as modules by
shelling out to the `moonlit` CLI (`exec` permission `["moonlit"]`).

## Config

No plugin-level config. No shared state.

## Middlewares

| Middleware | Config | Outputs | Behavior |
|---|---|---|---|
| `run-modules` | `modulePaths` (req, non-empty), `stages`, `continueOnModuleError` (default `false`), `arguments` | `results` (array of `{module, successful, durationMs}`), `failedCount` | Spawns `moonlit run -w <dir> [-f <file>] --output plain [-s <stage>]* [-a k=v]*` once per module path. |

### `modulePaths`

Each entry is either a directory (mapped to `-w <dir>` alone) or a `.yml`/
`.yaml` file (case-insensitive; split into `-w <parent-dir>` and
`-f <basename>`, with `-w .` when the file has no parent directory). An
empty `modulePaths` fails before any subprocess spawn with
`No module paths provided for run-modules.`

### `stages` and `arguments`

`stages` becomes one `-s <stage>` flag per entry, in order. `arguments` is a
map and becomes one `-a key=value` flag per entry, in sorted key order.

## Outputs

`results` is a JSON array with one entry per module attempted, each
`{module, successful, durationMs}`. `failedCount` is the number of modules
that did not succeed.

## Permissions

`exec: ["moonlit"]`

## Failure semantics

Two modes, controlled by `continueOnModuleError`:

- **Fail-fast (default, `false`)** — the first module that fails stops the
  run immediately; the middleware fails with
  `Module '<path>' failed with exit code <code>.` Later modules are never
  spawned.
- **Continue (`true`)** — every module runs regardless of prior failures.
  The middleware succeeds overall; per-module outcomes are reported in
  `results`, and the count of failed modules is reported in `failedCount`.

## Regenerate the committed artifact

    cargo build --manifest-path plugins/moonlit/Cargo.toml --target wasm32-wasip2 --release
    cp plugins/moonlit/target/wasm32-wasip2/release/moonlit.wasm engine/tests/fixtures/moonlit.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect engine/tests/fixtures/moonlit.wasm`.
