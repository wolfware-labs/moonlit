# moonlit-plugin-dotnet — Moonlit first-party dotnet plugin

Four middlewares — `build`, `pack`, `push`, `test` — built to a `wasm32-wasip2`
component. Shells out to the `dotnet` CLI (`exec` permission `["dotnet"]`, filesystem =
working directory). **Excluded from the workspace** so the engine build/CI never needs
the wasm target; native unit tests run via
`cargo test --manifest-path plugins/dotnet/Cargo.toml`.

## Regenerate the committed artifact

    cd plugins/dotnet
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/dotnet.wasm ../../engine/tests/fixtures/dotnet.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/dotnet.wasm`.

## Design notes

- **Output directories** are working-dir subdirs — `.moonlit/dotnet/<slug>/` (pack) and
  `.moonlit/dotnet-test/<slug>/` (test), wiped per run. A wasm component can only read
  back files inside its preopen and has no clock, so outputs go to a stable working-dir
  subdir rather than a host temp path. `<slug>` is the project's relative path minus its
  extension with separators flattened to `_` (e.g. `src/Api/Api.csproj` → `src_Api_Api`),
  so same-named projects in different directories don't share — and wipe — one output dir.
- **`packagePath`** is working-dir-relative and chains into `push`, which resolves it
  against the same working dir.
- **`push`** uses `dotnet nuget push` with `--timeout 30` and no `--skip-duplicate`.
  Errors collapse to two arms: auth (the `401 (`/`403 (` status form, or
  `Unauthorized`/`Forbidden`) → the frozen authentication message; otherwise a generic
  exit-code failure (the CLI exposes only exit code + text). The API key is passed as a
  CLI argument (`--api-key`), so it is visible in the host process listing
  (`/proc/<pid>/cmdline`) for the duration of the push; keep that in mind on shared hosts.
- **nupkg scan is sorted** for deterministic package selection.
- **`test`** outputs `passed`/`failed`/`skipped`/`total`; `skipped = total − executed`
  from the TRX `<Counters>` element.
