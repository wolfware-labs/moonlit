//! `send-notification` — posts a plain-text message to a Slack channel. Blank
//! channel/message fail before any HTTP request; no outputs.

use crate::api;
use crate::config::SlackPluginConfig;
use moonlit_plugin_sdk::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SendNotificationConfig {
    pub channel: String,
    pub message: String,
}

#[derive(Default)]
pub struct SendNotification;

impl Middleware for SendNotification {
    const NAME: &'static str = "send-notification";
    const DESCRIPTION: &'static str = "send a notification message to a Slack channel";
    type Config = SendNotificationConfig;

    fn execute(&self, ctx: &Context, cfg: SendNotificationConfig) -> MiddlewareResult {
        if cfg.channel.trim().is_empty() {
            return MiddlewareResult::failure("No Slack channel provided for notification.");
        }
        if cfg.message.trim().is_empty() {
            return MiddlewareResult::failure("No message provided for Slack notification.");
        }
        let token = ctx.plugin_config::<SlackPluginConfig>().token.clone();
        match api::post_message(ctx, &token, &cfg.channel, &cfg.message) {
            Ok(()) => MiddlewareResult::success(),
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn cfg(channel: &str, message: &str) -> SendNotificationConfig {
        SendNotificationConfig { channel: channel.into(), message: message.into() }
    }

    #[test]
    fn blank_channel_fails_before_request() {
        let host = MockHost::new();
        let pc = SlackPluginConfig { token: "t".into() };
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_plugin_config(&pc);
        let w = run(&SendNotification, &ctx, cfg("  ", "hi")).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("No Slack channel provided for notification.")
        );
        assert!(host.recorded_requests().is_empty());
    }

    #[test]
    fn blank_message_fails_before_request() {
        let host = MockHost::new();
        let pc = SlackPluginConfig { token: "t".into() };
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_plugin_config(&pc);
        let w = run(&SendNotification, &ctx, cfg("#x", "  ")).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("No message provided for Slack notification.")
        );
        assert!(host.recorded_requests().is_empty());
    }

    #[test]
    fn happy_path_posts_and_succeeds() {
        let host = MockHost::new().with_http_response(200, br#"{"ok":true}"#);
        let pc = SlackPluginConfig { token: "xoxb-t".into() };
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_plugin_config(&pc);
        assert!(run(&SendNotification, &ctx, cfg("#general", "hello")).is_success());
        assert_eq!(host.recorded_requests().len(), 1);
    }
}
