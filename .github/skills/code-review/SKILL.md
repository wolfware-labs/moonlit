---
name: code-review
description: Repository context for reviewing moonlit pull requests: the sandbox boundary, the WIT contract, release-commit semantics, and what makes a test here worth trusting.
---

# Reviewing moonlit

Moonlit runs release pipelines defined in YAML, executed by a Rust engine that
loads plugins as sandboxed WebAssembly components. The workspace lives under
`crates/`: `engine` (library), `cli` (the `moonlit` binary), and `pdk` /
`pdk-macros` (the plugin SDK, published to crates.io).

## Security boundary

`crates/engine/src/host/` is the plugin sandbox. Treat any change under it as
security-relevant and say so in the review:

- `perms.rs` builds the WASI context from the `filesystem` and `env` grants
  (`build_wasi_ctx`): a filtered environment and a single preopened directory.
  A widened grant changes what every plugin may do.
- `exec` is not a WASI permission, since WASI has no subprocess concept.
  `perms.rs` compiles the allowlist, `host/mod.rs` holds it as
  `HostState.exec_allow`, and `imports.rs` checks it before each spawn or run.
- `net.rs` allows or denies each outbound HTTP request against the plugin's
  `permissions.network` allowlist. The deny path must run before anything
  reaches a socket. A denial that only logs is a sandbox escape.

Plugins are untrusted. A change that lets one reach the filesystem, the network,
or a subprocess beyond its declared grants is a defect regardless of tests.

## The WIT contract

`crates/engine/wit/` is the single source of truth for the `moonlit:plugin` ABI.
`crates/pdk/wit/` is a verbatim copy, because a published crate cannot reference
a sibling by path; `scripts/check-wit-sync.sh` verifies the two trees match.

If a PR changes the WIT package version, `PLUGIN_WORLD` in
`crates/engine/src/publish.rs` must move with it, and the registry's
`SupportedWorld` has to move in the same deploy window. Flag that explicitly.

## Commit messages drive releases

On every push to `main`, semantic-release reads Conventional Commits, writes the
new version and changelog, and tags `moonlit-v<version>`. It publishes nothing.
The tag then triggers `release.yml`, which builds the artifacts and publishes
them to GitHub Releases, Homebrew, npm, Chocolatey and Docker Hub. The PDK
crates are versioned separately by `release-plz.yml`.

The `!` marker means breaking for a moonlit user, such as a changed CLI flag,
config schema, plugin ABI or default. It overrides the commit type, so
`chore(deps)!` cuts a major release of the CLI. Upgrading an internal dependency
is not a breaking change; flag any `!` that does not correspond to something a
user must react to.

Commits carry no AI or co-author attribution.

## Tests

The project is test-driven, and the bar is whether a test can fail. When
reviewing one, ask what edit to the production code it would catch. Tests that
assert a constant, or that a warning was logged without asserting the behaviour
it describes, have shipped here before and are worth calling out.

`crates/engine/tests/host_network.rs` is the model: it allowlists one host, requests
another, and asserts the mock server received **no request at all**.

## Dependencies

`wasmtime`, `wasmtime-wasi` and `wasmtime-wasi-http` must share an exact version,
and `wasmparser` / `wit-parser` must match what that wasmtime requires. A PR
moving one of them alone is wrong even when it builds.

Third-party Actions are pinned to a commit SHA with the version in a trailing
comment, and Dependabot moves both together. The local reusable workflows that
`release.yml` calls (`./.github/workflows/publish-*.yml`) are referenced by
relative path and travel with the commit, so they are not pinned and are not a
finding.
