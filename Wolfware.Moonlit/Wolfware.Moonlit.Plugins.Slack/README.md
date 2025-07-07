# Moonlit Slack Plugin

[![NuGet](https://img.shields.io/nuget/v/Wolfware.Moonlit.Plugins.Slack.svg)](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.Slack/)

## Overview

The Moonlit Slack Plugin provides Slack integration capabilities for the Moonlit release automation framework. This plugin allows your Moonlit applications to send notifications to Slack channels, keeping your team informed about release events and workflow operations.

## Features

- Send notifications to Slack channels about release events
- Integrate Slack messaging into your Moonlit pipeline
- Customizable message templates
- Simple configuration through your pipeline YAML
- Built on top of SlackNet for reliable Slack API operations

## Installation

Install the Moonlit Slack Plugin using the NuGet Package Manager:

```
Install-Package Wolfware.Moonlit.Plugins.Slack
```

Or via the .NET CLI:

```
dotnet add package Wolfware.Moonlit.Plugins.Slack
```

## Usage

### Basic Setup

Add the Slack plugin to your Moonlit release pipeline configuration:

```yaml
plugins:
  - name: "slack"
    url: "nuget://Wolfware.Moonlit.Plugins.Slack/1.0.0"
    configuration:
      token: "YOUR_SLACK_API_TOKEN"
```

### Available Middlewares

The plugin provides a middleware for sending notifications:

- **send-notification**: Sends a message to a specified Slack channel

### Sending Notifications

Configure the send-notification middleware in your pipeline:

```yaml
middlewares:
  - name: "send-notification"
    configuration:
      channel: "#releases"
      message: "🚀 New version ${NextVersion} has been released!"
```

## Requirements

- .NET 9.0 or higher
- Moonlit Plugins framework
- Slack API token with appropriate permissions
- SlackNet (automatically installed as a dependency)

## License

This project is licensed under the terms specified in the LICENSE.txt file included with this package.

## About Wolfware

Moonlit is a product of Wolfware LLC, providing modern tools for streamlined development workflows.

© 2025 Wolfware LLC. All rights reserved.