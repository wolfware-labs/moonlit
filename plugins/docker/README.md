# moonlit-plugin-docker — Moonlit first-party docker plugin

Four middlewares — `login`, `setup-buildx`, `build-and-push`, `deploy` — built to a
`wasm32-wasip2` component. Shells out to the `docker` CLI (`exec` permission
`["docker"]`, filesystem = working directory). One component instance per pipeline run
holds `DockerShared` (the buildx builder name recorded by `setup-buildx` and read by
`build-and-push`).

## Regenerate the committed artifact

    cd plugins/docker
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/docker.wasm ../../engine/tests/fixtures/docker.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/docker.wasm`.

## Design notes

- **Randomness** — where a value must be generated (the default buildx builder name in
  `setup-buildx`), it comes from the SDK's `ctx.random().uuid()`, which is itself backed
  by `Host::random_bytes`. A wasm component has no OS entropy source of its own, so all
  randomness is routed through the host.
- **Shared state** — `DockerShared.builder` is a `Shared<Option<String>>` set by
  `setup-buildx` (when `setBuilderVariable` is true) and read by `build-and-push`.
  `build-and-push` resolves the builder name in priority order: explicit `builder`
  config, then shared state, then the `MOONLIT_DOCKER_BUILDX_BUILDER` environment
  variable — first non-blank value wins.
- **Login password via stdin** — `login` never puts the password on the argv. It is
  piped in via `--password-stdin`, so it does not appear in the host process listing
  (`/proc/<pid>/cmdline`). Both `username` and `password` must be non-blank or the
  middleware fails before spawning a process.
- **`build-and-push` fixed argument order** — argv is built in a fixed sequence
  (`--builder`, `--tag`\*, `--file`, `--build-arg`\*, `--label`\*, `--platform`,
  `--no-cache`, `--cache-from`\*, `--cache-to`\*, `--pull`, `--push`/`--load`,
  `<context>`) per MVP_SPEC §11.7, so generated commands are deterministic and
  testable byte-for-byte. `push` defaults to `true` (`--push`); setting it to `false`
  switches to `--load`. `labels` is a `BTreeMap`, so `--label` flags are always emitted
  in sorted key order regardless of input order.
- **`deploy` is compose-only; swarm is a stub** — `service` being set is treated as a
  swarm deployment and fails immediately with `"Swarm deploys are not supported yet."`
  before any process is spawned; only the `docker compose` path is implemented for MVP.
  `environment` entries are exported as process environment variables on the spawned
  `docker compose` command alongside `DOCKER_HOST` (set from `host`), so they are
  visible to the compose file's variable substitution. `image` is accepted in
  `DeployConfig` but is inert in MVP — it is not referenced by `execute`; matching
  images to services is left to the compose file itself.
