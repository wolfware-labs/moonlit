//! Moonlit first-party `slack` plugin. Posts to the Slack Web API
//! (`chat.postMessage`) via the host HTTP capability; one middleware.

mod api;
mod config;
mod send_notification;

use moonlit_plugin_sdk::prelude::*;

use config::SlackPluginConfig;
use send_notification::SendNotification;

moonlit_plugin! {
    name: "slack",
    config: SlackPluginConfig,
    middlewares: [SendNotification],
}
