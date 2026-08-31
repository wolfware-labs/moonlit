# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-pdk-v0.3.0...moonlit-pdk-v0.3.1) - 2026-08-01

### Other

- *(sdk)* pin the crate-doc ABI strings to the shipped WIT
- *(sdk)* compile the README quickstart against the 0.3.0 API

## [0.3.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-pdk-v0.2.1...moonlit-pdk-v0.3.0) - 2026-07-31

### Added

- *(cli)* [**breaking**] declare middleware input and output schemas in inspect
- *(sdk)* [**breaking**] rename Config to Input, add typed Output, emit input/output schema (ABI 0.3.0)

## [0.2.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-pdk-v0.2.0...moonlit-pdk-v0.2.1) - 2026-07-31

### Fixed

- *(sdk)* derive JsonSchema on changelog Category and Entry

## [0.2.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-pdk-v0.1.1...moonlit-pdk-v0.2.0) - 2026-07-30

### Added

- *(sdk)* [**breaking**] require plugin ABI 0.2.0 (icon + middleware config schema)
- *(macro)* embed plugin icon and emit middleware config schemas
- *(sdk)* require JsonSchema on Middleware::Config
- *(wit)* bump plugin ABI to 0.2.0 with icon + middleware config-schema

### Other

- rustfmt Plan A files to satisfy CI fmt gate

## [0.1.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-pdk-v0.1.0...moonlit-pdk-v0.1.1) - 2026-07-29

### Other

- relicense from Elastic-2.0 to MIT OR Apache-2.0
- release v0.1.0 ([#1](https://github.com/wolfware-labs/moonlit/pull/1))

## [0.1.0](https://github.com/wolfware-labs/moonlit/releases/tag/moonlit-pdk-v0.1.0) - 2026-07-29

### Added

- *(sdk)* add monotonic sleep (Host::sleep_nanos, Clock::sleep_ms)
- *(sdk)* monotonic clock capability via ctx.clock()
- *(sdk)* host random_bytes capability and Context::uuid
- *(semantic-release)* changelog generator; sdk changelog gains Serialize
- *(github)* align create-release comment and changelog whitespace to 1.x
- *(sdk)* add changelog markdown generator
- *(sdk)* add PluginConfig init-validation hook
- *(sdk)* add state::Shared cell for mutable plugin shared state
- *(sdk)* expose MiddlewareResult accessors for testing assertions
- *(sdk)* add sdk::http blocking client with gzip and timeouts
- *(sdk)* add sdk::process command/stream/spawn wrappers
- *(sdk)* add sdk::env and relocate RealHost to a wasm-only module
- *(sdk)* add moonlit_plugin! proc-macro
- *(sdk)* add Middleware trait and native run() harness
- *(sdk)* add Host bridge, Context, and MockHost test harness
- *(sdk)* add §5.4 coercion config deserializer
- *(sdk)* add MiddlewareResult and Output builders
- *(sdk)* scaffold plugin-sdk facade with vendored WIT bindings

### Fixed

- *(cli)* inspect required-config plugins via config-free describe export

### Other

- *(sdk)* dual-license under MIT OR Apache-2.0 and complete crates.io metadata
- rename SDK crates from moonlit-plugin-sdk to moonlit-pdk
- *(sdk)* use a real config struct in the README example
- *(sdk)* add crates.io READMEs for the SDK crates
- *(workspace)* decouple SDK crate versions and mark apps unpublishable
- *(sdk)* regroup randomness under ctx.random()
- *(sdk)* cover moonlit_plugin! state and config expansion
