# Moonlit GitHub Plugin

[![NuGet](https://img.shields.io/nuget/v/Wolfware.Moonlit.Plugins.GitHub.svg)](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.GitHub/)

## Overview

The Moonlit GitHub Plugin provides GitHub integration capabilities for the Moonlit release automation framework. This
plugin allows your Moonlit applications to interact with GitHub repositories, manage releases, track issues, and
incorporate GitHub-based workflows into your release pipelines.

## Features

- Retrieve latest tags from GitHub repositories
- Create GitHub releases from your Moonlit pipeline
- Get commit history and changes between releases
- Write GitHub-related variables to your release context
- Built on top of Octokit for reliable GitHub API operations

## Installation

Install the Moonlit GitHub Plugin using the NuGet Package Manager:

```
Install-Package Wolfware.Moonlit.Plugins.GitHub
```

Or via the .NET CLI:

```
dotnet add package Wolfware.Moonlit.Plugins.GitHub
```

## Usage

### Basic Setup

Add the GitHub plugin to your Moonlit release pipeline configuration:

```yaml
plugins:
  - name: "github"
    url: "nuget://Wolfware.Moonlit.Plugins.GitHub/1.0.0"
    config:
      token: "YOUR_GITHUB_TOKEN"
```

### Available Middlewares

The plugin provides several middlewares that you can use in your pipeline:

- **latest-tag**: Retrieves the latest tag from your GitHub repository
- **items-since-commit**: Gets all items (commits, issues, etc.) since a specific commit
- **create-release**: Creates a new release in GitHub
- **write-variables**: Writes GitHub-related variables to your pipeline context

## Requirements

- .NET 9.0 or higher
- Moonlit Plugins framework
- Wolfware.Moonlit.Plugins.Git (automatically installed as a dependency)
- Octokit (automatically installed as a dependency)

## License

This project is licensed under the terms specified in the LICENSE.txt file included with this package.

## About Wolfware

Moonlit is a product of Wolfware LLC, providing modern tools for streamlined development workflows.

© 2025 Wolfware LLC. All rights reserved.