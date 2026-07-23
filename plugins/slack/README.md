# moonlit-plugin-slack

First-party Moonlit plugin. Posts messages to Slack via the Web API
(`chat.postMessage`) over the host HTTP capability.

## Config

Plugin-level: `token` (required) — a Slack bot/user OAuth token. Missing or
blank fails at init with `Slack API token is required.`

## Middlewares

| Middleware | Config | Outputs | Behavior |
|---|---|---|---|
| `send-notification` | `channel` (req), `message` (req) | — | `POST chat.postMessage {channel, text}`. Blank channel/message fail before any request; a Slack `ok:false` response fails with the Slack error code. |

## Permissions

`network: ["slack.com"]`

## Regenerate the committed artifact

    cd plugins/slack
    cargo build --target wasm32-wasip2 --release
    cp target/wasm32-wasip2/release/slack.wasm ../../engine/tests/fixtures/slack.wasm

Requires: `rustup target add wasm32-wasip2`. Verify with
`moonlit plugin inspect ../../engine/tests/fixtures/slack.wasm`.

## Design notes

- **Always-200 API** — Slack returns HTTP 200 even for logical failures,
  encoding the outcome in a JSON `ok` field. The client parses the body and
  branches on `ok`, surfacing the Slack `error` code on failure.
- **No state, no outputs** — `send-notification` is fire-and-report; it holds
  no shared state and emits no output values.
