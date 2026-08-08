# Changelog

All notable changes to the Moonlit CLI are documented here.

## [1.1.6](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.5...moonlit-v1.1.6) (2026-08-08)

### Bug Fixes

* **choco:** give the Chocolatey package an icon and full metadata ([77b6e5b](https://github.com/wolfware-labs/moonlit/commit/77b6e5b9ac968f9399a95fdd79726c6dc25c438c))

## [1.1.5](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.4...moonlit-v1.1.5) (2026-08-05)

### Bug Fixes

* **cli:** resolve plugin artifacts from cargo metadata ([9ca348a](https://github.com/wolfware-labs/moonlit/commit/9ca348a99ed5bd033777f53d4b71b67b3220393b))

## [1.1.4](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.3...moonlit-v1.1.4) (2026-08-03)

### Bug Fixes

* **engine:** stamp the plugin world this engine actually ships ([a6e5fb2](https://github.com/wolfware-labs/moonlit/commit/a6e5fb22d32cc25973a8a1ebe8af47399d56593b))

## [1.1.3](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.2...moonlit-v1.1.3) (2026-08-01)

### Bug Fixes

* **ci:** let the tolerated Chocolatey 403 actually pass the job ([08d0fb1](https://github.com/wolfware-labs/moonlit/commit/08d0fb10576e7b12a883aeaf4d00a5ede5f0f253))

## [1.1.2](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.1...moonlit-v1.1.2) (2026-08-01)

### Bug Fixes

* **cli:** do not promise a server-side revoke logout cannot always do ([355f34c](https://github.com/wolfware-labs/moonlit/commit/355f34ceb2b8c371bc1cf3cf921b9cc3322252ba))

## [1.1.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.0...moonlit-v1.1.1) (2026-08-01)

### Bug Fixes

* **login:** bound registry requests with request and connect timeouts ([debb9c1](https://github.com/wolfware-labs/moonlit/commit/debb9c17469da6c2bd16b97b1c61671c64a9edcd))
* **logout:** remove the local credential for Basic logins ([e025b0f](https://github.com/wolfware-labs/moonlit/commit/e025b0fed7d286dcac710cca643a099d7f11a921))

## [1.1.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.0.0...moonlit-v1.1.0) (2026-08-01)

### Features

* **login:** default to RFC 8628 device flow, keep --token for CI ([3a457f3](https://github.com/wolfware-labs/moonlit/commit/3a457f3c2d5d6e08869bdd17f7e11d8b7fc6f31f))
* **login:** optional host defaulting to registry.moonlitbuild.dev ([892f332](https://github.com/wolfware-labs/moonlit/commit/892f332ce81a6ece5044289c4e22fe995391a312))
* **logout:** revoke server-side and remove local credential ([9c29297](https://github.com/wolfware-labs/moonlit/commit/9c2929705336d84e39f50e04ca87f8c0b00634a9))

### Bug Fixes

* **login:** harden device flow — home-dir, atomic 0600, URL scheme, timeout, retries ([fc35e3e](https://github.com/wolfware-labs/moonlit/commit/fc35e3e31fc4275e2de5f81158459c7c9bf70ce8))
* **login:** match loopback host exactly so lookalikes aren't downgraded to http ([a1df84a](https://github.com/wolfware-labs/moonlit/commit/a1df84a8cb3726b3a7f2cd41cbab3683f26a0d6a))

## [1.0.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v0.2.0...moonlit-v1.0.0) (2026-07-31)

### ⚠ BREAKING CHANGES

* **cli:** `moonlit plugin inspect --output json` replaces each
middleware's `configSchema` field with `inputSchema` and `outputSchema`.
* **sdk:** moonlit:plugin world 0.2.0 -> 0.3.0. Middleware::Config renamed to
Input; new Middleware::Output. middleware-info.config-schema renamed to input-schema and
output-schema added. MiddlewareResult is now generic (MiddlewareResult<Output>) with ok()/
failure(); success()/success_with() removed. SDK adds NoInput/NoOutput markers.

### Features

* **cli:** declare middleware input and output schemas in inspect ([76ce63f](https://github.com/wolfware-labs/moonlit/commit/76ce63fcb3310026a2d325fb993feb172540c68d))
* **sdk:** rename Config to Input, add typed Output, emit input/output schema (ABI 0.3.0) ([8bac9d5](https://github.com/wolfware-labs/moonlit/commit/8bac9d560e2140cf89f5d6971b3f519b48a7390f))

### Bug Fixes

* **sdk:** derive JsonSchema on changelog Category and Entry ([2cae64e](https://github.com/wolfware-labs/moonlit/commit/2cae64e0ff1fdf088d18ca1c20acf0d525a4adb6))

## [0.2.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v0.1.1...moonlit-v0.2.0) (2026-07-30)

### ⚠ BREAKING CHANGES

* **sdk:** plugin ABI moonlit:plugin bumped 0.1.0 -> 0.2.0; Middleware::Config now requires schemars::JsonSchema.

### Features

* **cli:** scaffold plugins with JsonSchema config for SDK 0.2.0 ([62bb81c](https://github.com/wolfware-labs/moonlit/commit/62bb81c8e4fbda2d9820cab0c0e6bce5e1bdf2c3))
* **cli:** surface icon + middleware config schema in plugin inspect ([962663f](https://github.com/wolfware-labs/moonlit/commit/962663f775dd713e74ae57bf6c1364b1b12c33c2))
* **engine:** carry icon + config schema through host metadata ([bce5ec1](https://github.com/wolfware-labs/moonlit/commit/bce5ec128c21bc22aef3647820e2df241328a401))
* **macro:** embed plugin icon and emit middleware config schemas ([4f4585c](https://github.com/wolfware-labs/moonlit/commit/4f4585c13cfb050583926d467157ecdf4185d11e))
* **sdk:** require JsonSchema on Middleware::Config ([340aee7](https://github.com/wolfware-labs/moonlit/commit/340aee79bee39cfbb48bbc6fbec1585976aac446))
* **sdk:** require plugin ABI 0.2.0 (icon + middleware config schema) ([cd22a17](https://github.com/wolfware-labs/moonlit/commit/cd22a17656da2e0f607fed3b55a0c6d8b6f54dca))
* **wit:** bump plugin ABI to 0.2.0 with icon + middleware config-schema ([b5e8543](https://github.com/wolfware-labs/moonlit/commit/b5e8543df100a54aed5d474d80ae3ec03e3368ae))

## 0.1.1

- `moonlit version` now renders the wolf-and-moon logo with the wordmark,
  slogan, version, credits, and a clickable docs link on a truecolor terminal,
  falling back to a plain banner when color is unavailable.
