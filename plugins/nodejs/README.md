# moonlit-plugin-nodejs — Moonlit first-party nodejs plugin

Six middlewares — `install`, `run-script`, `build`, `pack`, `push`, `test` — built to a
`wasm32-wasip2` component. Shells out to the `npm` CLI (`exec` permission `["npm",
"node"]`, filesystem = working directory). **Excluded from the workspace** so the engine
build/CI never needs the wasm target; native unit tests run via
`cargo test --manifest-path plugins/nodejs/Cargo.toml`.

There is no 1.x behavior to match: the 1.x NodeJs middlewares were unimplemented stubs,
so this plugin implements MVP_SPEC §11.6 directly.

## Regenerate the committed artifact

    cd plugins/nodejs
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/nodejs.wasm ../../engine/tests/fixtures/nodejs.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/nodejs.wasm`.

## Design notes

- **npm only** — the `packageManager` field (yarn/pnpm) is not consulted (MVP scope).
- **`package.json` pre-flight** — `install`/`run-script`/`build`/`pack`/`test` fail with
  `package.json not found in directory: <dir>` before invoking npm, giving a branded
  message and a deterministic offline failure path.
- **`pack` output** — the tarball is written to `.moonlit/npm-pack/` under the middleware
  `directory` (wiped per run), or an explicit `destination` (created if missing, not
  wiped). The emitted `packagePath` is **working-dir-relative** so it chains into `push`.
  A wasm component has no clock and can only read files inside its preopen, so the
  spec's `<tmp>/npm-packs/<name>/<timestamp>/` default is replaced by this scheme.
  `destination` is interpreted as a working-dir-relative path; an absolute path is out
  of contract under the wasm filesystem sandbox.
- **`push` auth** — the token is written to a scoped `.npmrc` under `.moonlit/npm-push/`
  (wiped per run) and passed via `--userconfig`; it never appears on the argv, so it is
  not visible in the host process listing (`/proc/<pid>/cmdline`). A blank token (local
  and plugin config) fails fast with a branded message. Non-zero exits classify to
  `Version already published.` (`EPUBLISHCONFLICT` / cannot-publish-over) or an
  authentication error (`401 (`/`403 (`, `E401`, `ENEEDAUTH`, unauthorized/forbidden);
  everything else is a generic exit-code failure.
- **`test`** is new in 2.0 (spec's optional stretch), mirroring `dotnet.test`.
