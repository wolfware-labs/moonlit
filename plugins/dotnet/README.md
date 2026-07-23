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

## 2.0 vs 1.x divergences

- **Output directories** are working-dir subdirs — `.moonlit/dotnet/<slug>/` (pack) and
  `.moonlit/dotnet-test/<slug>/` (test), wiped per run — not host `Path.GetTempPath()`:
  a wasm component can only read back files inside its preopen, and there is no clock
  for the 1.x `yyyyMMddHHmmss` subdir. `<slug>` is the project's relative path minus its
  extension with separators flattened to `_` (e.g. `src/Api/Api.csproj` → `src_Api_Api`),
  so same-named projects in different directories don't share — and wipe — one output dir.
- **`packagePath`** is working-dir-relative (was an absolute host path); it still chains
  into `push`, which resolves relative to the same working dir.
- **`push`** uses `dotnet nuget push` (was NuGet.Protocol), `--timeout 30`, no
  `--skip-duplicate`. Errors collapse to two arms: auth (the `401 (`/`403 (` status form,
  or `Unauthorized`/`Forbidden`) → the frozen authentication message; otherwise a generic
  exit-code failure. 1.x's separate "Network error" arm is dropped (the CLI exposes only
  exit code + text). The API key is passed as a CLI argument (`--api-key`), so — unlike
  1.x's in-process NuGet.Protocol push — it is visible in the host process listing
  (`/proc/<pid>/cmdline`) for the duration of the push; keep that in mind on shared hosts.
- **nupkg scan is sorted** for determinism (1.x relied on unspecified `Directory.GetFiles`
  order).
- **`test`** is newly implemented (1.x threw `NotImplementedException`). Outputs
  `passed`/`failed`/`skipped`/`total`; `skipped = total − executed` from the TRX
  `<Counters>` element.
