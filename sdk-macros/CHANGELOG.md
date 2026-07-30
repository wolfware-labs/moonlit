# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-sdk-macros-v0.1.1...moonlit-sdk-macros-v0.2.0) - 2026-07-30

### Added

- *(sdk)* [**breaking**] require plugin ABI 0.2.0 (icon + middleware config schema)
- *(macro)* embed plugin icon and emit middleware config schemas

### Other

- rustfmt Plan A files to satisfy CI fmt gate

## [0.1.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-sdk-macros-v0.1.0...moonlit-sdk-macros-v0.1.1) - 2026-07-29

### Other

- relicense from Elastic-2.0 to MIT OR Apache-2.0
- release v0.1.0 ([#1](https://github.com/wolfware-labs/moonlit/pull/1))

## [0.1.0](https://github.com/wolfware-labs/moonlit/releases/tag/moonlit-sdk-macros-v0.1.0) - 2026-07-29

### Added

- *(sdk)* add monotonic sleep (Host::sleep_nanos, Clock::sleep_ms)
- *(sdk)* monotonic clock capability via ctx.clock()
- *(sdk)* host random_bytes capability and Context::uuid
- *(sdk)* add PluginConfig init-validation hook
- *(sdk)* add sdk::http blocking client with gzip and timeouts
- *(sdk)* add sdk::process command/stream/spawn wrappers
- *(sdk)* add sdk::env and relocate RealHost to a wasm-only module
- *(sdk)* add moonlit_plugin! proc-macro

### Fixed

- *(cli)* inspect required-config plugins via config-free describe export

### Other

- *(sdk)* dual-license under MIT OR Apache-2.0 and complete crates.io metadata
- rename SDK crates from moonlit-plugin-sdk to moonlit-sdk
- *(sdk)* add crates.io READMEs for the SDK crates
- *(workspace)* decouple SDK crate versions and mark apps unpublishable
