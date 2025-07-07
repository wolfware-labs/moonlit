# Moonlit Plugins

[![NuGet](https://img.shields.io/nuget/v/Wolfware.Moonlit.Plugins.svg)](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins/)

## Overview

Moonlit Plugins is the core framework for building extensible plugins for the Moonlit release automation system. This
package provides the foundation for creating custom plugins that integrate seamlessly with the Moonlit pipeline,
offering a standardized approach to extending functionality.

## Features

- Extensible plugin architecture for Moonlit applications
- Core abstractions for creating custom release pipeline middlewares
- Standardized context and result handling for plugin operations
- Service registration patterns for dependency injection
- Simplified configuration binding and management
~~~~
## Installation

Install the Moonlit Plugins package using the NuGet Package Manager:

```
Install-Package Wolfware.Moonlit.Plugins
```

Or via the .NET CLI:

```
dotnet add package Wolfware.Moonlit.Plugins
```

## Usage

### Creating a Custom Plugin

Implement a plugin by extending the `PluginStartup` base class:

```csharp
public class MyCustomPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    // Register your services and configure options here
  }

  protected override void AddMiddlewares(IServiceCollection services)
  {
    // Register your middlewares here
    services.AddMiddleware<MyCustomMiddleware>("my-custom-action");
  }
}
```

### Implementing a Middleware

Create custom middleware components to extend the pipeline functionality:

```csharp
public class MyCustomMiddleware : ReleaseMiddleware<MyConfig>
{
  protected override Task<MiddlewareResult> ExecuteAsync(
    ReleaseContext context, 
    MyConfig configuration)
  {
    // Implement your middleware logic here
    return Task.FromResult(MiddlewareResult.Success());
  }
}
```

### Pipeline Integration

Plugins created with this framework can be easily integrated into Moonlit release pipelines:

```yaml
plugins:
  - name: "my-custom-plugin"
    url: "nuget://MyCompany.Moonlit.CustomPlugin/1.0.0"

middlewares:
  - name: "my-custom-action"
    configuration:
    # Your middleware configuration
```

## Available Plugins

The Moonlit ecosystem includes several official plugins built on this framework:

- [Moonlit Git Plugin](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.Git/)
- [Moonlit GitHub Plugin](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.GitHub/)
- [Moonlit Slack Plugin](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.Slack/)
- [Moonlit Semantic Release Plugin](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.SemanticRelease/)

## Requirements

- .NET 9.0 or higher
- Microsoft.Extensions.Configuration.Abstractions
- Microsoft.Extensions.Logging.Abstractions
- Microsoft.Extensions.Options.ConfigurationExtensions

## License

This project is licensed under the terms specified in the LICENSE.txt file included with this package.

## About Wolfware

Moonlit is a product of Wolfware LLC, providing modern tools for streamlined development workflows.

© 2025 Wolfware LLC. All rights reserved.
