---
title: Plugins Overview
description: Overview of the official plugins available for Moonlit
---

# Plugins Overview

Moonlit's functionality is extended through plugins. This page provides an overview of the official plugins available for Moonlit.

## What are Plugins?

Plugins are NuGet packages that add functionality to Moonlit. Each plugin provides one or more middlewares that can be used in your pipeline steps. For more information about how plugins work, see the [Plugins System](../guide/concepts/plugins.md) page.

## Official Plugins

Moonlit comes with several official plugins that provide integration with common tools and services:

### Git Plugin

**Package**: `Wolfware.Moonlit.Plugins.Git`

The Git plugin provides integration with Git repositories. It allows you to:

- Get information about the current repository
- Perform Git operations like commit, tag, and push

[Learn more about the Git Plugin](./git.md)

### GitHub Plugin

**Package**: `Wolfware.Moonlit.Plugins.Github`

The GitHub plugin provides integration with GitHub. It allows you to:

- Create and manage releases
- Work with issues and pull requests
- Get information about tags and commits

[Learn more about the GitHub Plugin](./github.md)

### Semantic Release Plugin

**Package**: `Wolfware.Moonlit.Plugins.SemanticRelease`

The Semantic Release plugin provides tools for semantic versioning. It allows you to:

- Calculate the next version based on commit messages
- Generate changelogs
- Apply version numbers to your project

[Learn more about the Semantic Release Plugin](./semantic-release.md)

### Slack Plugin

**Package**: `Wolfware.Moonlit.Plugins.Slack`

The Slack plugin provides integration with Slack. It allows you to:

- Send notifications to Slack channels
- Create and update messages
- Add reactions and attachments

[Learn more about the Slack Plugin](./slack.md)

### Dotnet Plugin

**Package**: `Wolfware.Moonlit.Plugins.Dotnet`

The Dotnet plugin provides integration with .NET projects. It allows you to:

- Build .NET projects without packing them
- Pack .NET projects into NuGet packages
- Push packages to NuGet repositories
- Manage package versions

[Learn more about the Dotnet Plugin](./dotnet.md)

### Docker Plugin

**Package**: `Wolfware.Moonlit.Plugins.Docker`

The Docker plugin provides integration with Docker. It allows you to:

- Build Docker images
- Push images to registries
- Manage tags and versions

[Learn more about the Docker Plugin](./docker.md)

### NodeJs Plugin

**Package**: `Wolfware.Moonlit.Plugins.NodeJs`

The NodeJs plugin provides integration with Node.js and NPM. It allows you to:

- Run any scripts defined in package.json
- Build Node.js projects without packing them
- Pack and publish NPM packages to registries
- Manage package versions
- Execute any Node.js-related operations

[Learn more about the NodeJs Plugin](./nodejs.md)

## Using Plugins

To use a plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
```

Then, you can use the middlewares provided by the plugin in your pipeline steps:

```yaml
stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: gh.latest-tag
```

## Plugin Configuration

Each plugin can be configured at two levels:

1. **Global Configuration**: Applied to the entire plugin
2. **Step Configuration**: Applied to a specific step

For more information about plugin configuration, see the [Configuration](../guide/concepts/configuration.md) page.

## Creating Custom Plugins

If the official plugins don't meet your needs, you can create your own custom plugins. For more information, see the [Plugin Development Reference](../reference/plugin-development.md) page.

## Examples

For examples of how to use plugins in your pipelines, see the following pages:

- [NuGet Release Pipeline](./examples/nuget-release.md)
- [Docker Deployment](./examples/docker-deployment.md)

## Next Steps

- Learn about specific plugins by clicking on their links above
- Explore the [configuration file structure](../reference/config-file.md)
- See how to [create your own plugins](../reference/plugin-development.md)
