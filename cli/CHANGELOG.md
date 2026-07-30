# Changelog

All notable changes to the Moonlit CLI are documented here.

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
