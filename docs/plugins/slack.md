---
title: Slack Plugin
description: Documentation for the Slack plugin in Moonlit
---

# Slack Plugin

The Slack plugin provides integration with Slack. It allows you to send notifications to Slack channels about your release process.

## Installation

To use the Slack plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "slack"
    url: "nuget://Wolfware.Moonlit.Plugins.Slack/1.0.0"
    config:
      token: $(SLACK_TOKEN)
```

Note that the Slack plugin requires a Slack token to authenticate with the Slack API. You can set this token as an environment variable and reference it in your configuration file.

## Middlewares

The Slack plugin provides the following middlewares:

### send-notification

The `send-notification` middleware sends a notification to a Slack channel.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| channel | string | Yes | - | The Slack channel to send the notification to (e.g., "#releases") |
| message | string | Yes | - | The message to send to the channel |

#### Outputs

This middleware does not produce any outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  notify:
    - name: notifySlackChannel
      run: "slack.send-notification"
      config:
        channel: "#releases"
        message: ":rocket:   New Release - <$(output:createRelease:url)|$(output:createRelease:name)>   :tada:"
```

In this example, the `send-notification` middleware sends a notification to the "#releases" channel with a message that includes a link to a GitHub release. The message uses Slack's markdown syntax to create a clickable link.

## Usage in Pipelines

The Slack plugin is commonly used in the final stages of a pipeline to notify team members about successful releases. It can be used to:

1. Announce new releases with links to release notes
2. Notify about build failures
3. Share deployment status updates

For a complete example of using the Slack plugin in a pipeline, see the [NuGet Release Pipeline](./examples/nuget-release.md) and [Docker Deployment](./examples/docker-deployment.md) examples.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [GitHub Plugin](./github.md) for GitHub API integration
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins