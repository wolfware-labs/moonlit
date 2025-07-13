---
title: Plugins System
description: Learn how Moonlit's plugin system works and how plugins extend functionality
---

# Plugins System

Moonlit's plugin system is one of its core features, allowing you to extend the tool's functionality through modular components. This page explains how the plugin system works and how plugins are used in Moonlit.

## What are Plugins?

In Moonlit, plugins are NuGet packages that provide additional functionality to your release pipeline. Each plugin can provide:

- **Middlewares**: Components that execute specific tasks in your pipeline
- **Services**: Reusable functionality that can be shared between middlewares
- **Configuration**: Settings that control how the plugin behaves

## Plugin Structure

A Moonlit plugin typically consists of:

1. **Startup Class**: Implements `IPluginStartup` or inherits from `PluginStartup`
2. **Middleware Classes**: Implement specific functionality for pipeline steps
3. **Service Classes**: Provide shared functionality
4. **Configuration Classes**: Define the configuration options for the plugin

### Startup Class

The startup class is the entry point for a plugin. It must implement the `IPluginStartup` interface or inherit from the `PluginStartup` class. This class is responsible for:

- Registering middlewares
- Registering services
- Configuring the plugin

Here's a simplified example of a plugin startup class:

```csharp
public sealed class GitPluginStartup : PluginStartup
{
    protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
    {
        // Register services
        services.AddSingleton<IGitService, GitService>();
    }

    protected override void AddMiddlewares(IServiceCollection services)
    {
        // Register middlewares with names
        services.AddMiddleware<RepoContextMiddleware>("repo-context");
    }
}
```

## Plugin Registration

Plugins are registered in your Moonlit configuration file under the `plugins` section:

```yaml
plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
```

Each plugin entry includes:

- **name**: A unique identifier for the plugin
- **url**: The NuGet package URL
- **config** (optional): Configuration settings for the plugin

## Plugin Loading Process

When Moonlit runs, it follows these steps to load plugins:

1. **Parse Plugin Definitions**: Read the plugin entries from the configuration file
2. **Resolve NuGet Packages**: Download and extract the specified NuGet packages
3. **Load Assemblies**: Load the plugin assemblies into the application domain
4. **Find Startup Classes**: Locate classes that implement `IPluginStartup`
5. **Initialize Plugins**: Call the startup methods to register services and middlewares
6. **Configure Plugins**: Apply the configuration from the YAML file

## Official Plugins

Moonlit comes with several official plugins:

- **Wolfware.Plugins.Git**: Git repository operations
- **Wolfware.Plugins.GitHub**: GitHub API integration
- **Wolfware.Plugins.SemanticRelease**: Semantic versioning and changelog generation
- **Wolfware.Plugins.Slack**: Slack notifications
- **Wolfware.Plugins.Dotnet**: .NET project operations
- **Wolfware.Plugins.Docker**: Docker image building and publishing
- **Wolfware.Plugins.NodeJs**: NPM package operations

Each plugin provides specific middlewares that you can use in your pipeline steps.

## Using Plugin Middlewares

Once a plugin is loaded, you can use its middlewares in your pipeline steps:

```yaml
stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: gh.latest-tag
```

The `run` property uses the format `pluginName.middlewareName` to specify which middleware to execute.

## Plugin Configuration

Plugins can be configured at two levels:

1. **Global Configuration**: Applied to the entire plugin
2. **Step Configuration**: Applied to a specific step

### Global Configuration

Global configuration is specified in the plugin definition:

```yaml
plugins:
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
```

### Step Configuration

Step configuration is specified in the step definition:

```yaml
stages:
  analyze:
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"
```

## Next Steps

- Learn about [Stages and Steps](./stages-steps.md) in more detail
- Explore the [Configuration](./configuration.md) options
- See how to [create your own plugins](../advanced/custom-plugins.md)
