# moonlit-plugin-semantic-release — Moonlit first-party semantic-release plugin

Three middlewares — `analyze`, `calculate-version`, `generate-changelog` — built to a
`wasm32-wasip2` component. Pure-Rust and **offline by default** — no `exec`/`filesystem`
permissions, and no network access unless you opt into AI-assisted changelog refinement
(see [AI-assisted changelog refinement](#ai-assisted-changelog-refinement)). **Excluded
from the workspace** so the engine build/CI never needs the wasm target; native unit tests
run via `cargo test --manifest-path plugins/semantic-release/Cargo.toml`.

## Regenerate the committed artifact

    cd plugins/semantic-release
    moonlit plugin build --release        # or: cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/semantic_release.wasm ../../engine/tests/fixtures/semantic-release.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/semantic-release.wasm`.

## AI-assisted changelog refinement

`generate-changelog` can optionally use an LLM to (a) drop non-user-facing commits and
(b) rewrite terse commit summaries into readable changelog entries. Both are **off by
default** — the plugin stays fully offline until you enable them. Turn them on with the
step flags `filterNonUserFacingCommits` and/or `refineCommitsSummary`, and provide an
`ai` block in the plugin `config`.

Commits are processed in **sequential batches of 15** (the wasm SDK is synchronous — no
concurrency). Each provider call retries on rate-limit/transport errors with backoff
(`Retry-After` hint, else `2^attempt × 500ms`, capped at 60s, `maxRetries` attempts).
Auth failures, malformed responses, and exhausted retries **fail the step** — the model is
never allowed to silently degrade the changelog.

### `ai` config options

| Key | Required | Default | Notes |
| --- | --- | --- | --- |
| `provider` | no | `openai` | One of `openai`, `anthropic`, `gemini`. |
| `apiKey` | **yes** | — | Use `$(...)` substitution (e.g. `$(OPENAI_API_KEY)`); read at plugin load, so **no `env` grant is needed**. |
| `model` | no | per provider | `openai` → `gpt-5-mini`, `anthropic` → `claude-haiku-4-5`, `gemini` → `gemini-2.5-flash`. |
| `baseUrl` | no | provider default | Override the API host (proxy / OpenAI-compatible gateway). Its host must also be in the `network` grant. |
| `maxRetries` | no | `5` | Retry attempts for rate-limit/transport failures. |
| `maxTokens` | no | `4096` | Max output tokens. **Anthropic only** (required by its API); ignored by OpenAI and Gemini. |

### `network` permission per provider

The plugin reaches exactly one host, so grant only that provider's host (plus any custom
`baseUrl` host):

| Provider | Host to grant |
| --- | --- |
| `openai` | `api.openai.com` |
| `anthropic` | `api.anthropic.com` |
| `gemini` | `generativelanguage.googleapis.com` |

The API key travels only in the provider's auth header (`Authorization: Bearer …` for
OpenAI, `x-api-key` for Anthropic, `x-goog-api-key` for Gemini) — never in a URL or log.

### Example

```yaml
plugins:
  - name: sr
    url: oci://registry.moonlitbuild.dev/wolfware/semantic-release:1.0.0
    config:
      ai:
        provider: anthropic
        apiKey: $(ANTHROPIC_API_KEY)   # no `env` grant required
        model: claude-haiku-4-5        # optional; shown value is the default
    permissions:
      network: ["api.anthropic.com"]   # only the chosen provider's host

stages:
  release:
    - name: changelog
      run: sr.generate-changelog
      config:
        filterNonUserFacingCommits: true
        refineCommitsSummary: true
```

## 2.0 vs 1.x divergences

- `prereleaseMappings` glob keys resolve by exact-key-then-alphabetical-glob
  precedence: engine config maps are unordered, so 1.x's declaration-order
  glob matching is not available.
- Rule-config enum values (`VersionBumpType`) are case-sensitive PascalCase
  (e.g. `Major`, `Minor`, `Patch`, `None`), unlike 1.x's case-insensitive binder.
- Custom `ChangelogRule` entries must specify `icon`, `section`, and `summary`
  explicitly — there are no silent per-property defaults; a rule missing one
  of these fails to deserialize instead of falling back to a default value.
- Malformed config values (e.g. an invalid semver in a prerelease mapping, or
  a non-ASCII sha) surface as a run failure rather than being silently
  coerced or skipped.
