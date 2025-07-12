---
title: Plugin Development Reference
description: Learn how to create custom plugins for Moonlit
---

# Plugin Development Reference

This page provides detailed information on how to create custom plugins for Moonlit. For a conceptual overview of the plugin system, see the [Plugins System](../guide/concepts/plugins.md) page.

## Plugin Structure

A Moonlit plugin is a .NET library packaged as a NuGet package. It consists of:

1. **Startup Class**: The entry point for your plugin
2. **Middleware Classes**: Components that execute specific tasks in the pipeline
3. **Service Classes**: Reusable functionality that can be shared between middlewares
4. **Configuration Classes**: Define the configuration options for your plugin

## Creating a Plugin Project

To create a new plugin, follow these steps:

1. Create a new .NET Class Library project:

```bash
dotnet new classlib -n MyCompany.Moonlit.Plugins.MyPlugin
```

2. Add a reference to the Wolfware.Moonlit.Plugins package:

```bash
dotnet add package Wolfware.Moonlit.Plugins
```

3. Create a startup class that implements `IPluginStartup` or inherits from `PluginStartup`.

## Plugin Startup Class

The startup class is the entry point for your plugin. It must implement the `IPluginStartup` interface or inherit from the `PluginStartup` class.

```csharp
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins;

namespace MyCompany.Moonlit.Plugins.MyPlugin
{
    public sealed class MyPluginStartup : PluginStartup
    {
        protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
        {
            // Register services
            services.AddSingleton<IMyService, MyService>();
        }

        protected override void AddMiddlewares(IServiceCollection services)
        {
            // Register middlewares with names
            services.AddMiddleware<MyMiddleware>("my-middleware");
        }
    }
}
```

### ConfigurePlugin Method

The `ConfigurePlugin` method is where you register your services with the dependency injection container. This method is called when the plugin is loaded.

```csharp
protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
{
    // Register services
    services.AddSingleton<IMyService, MyService>();

    // Bind configuration
    services.Configure<MyPluginOptions>(configuration);
}
```

### AddMiddlewares Method

The `AddMiddlewares` method is where you register your middlewares with the dependency injection container. This method is called after `ConfigurePlugin`.

```csharp
protected override void AddMiddlewares(IServiceCollection services)
{
    // Register middlewares with names
    services.AddMiddleware<MyMiddleware>("my-middleware");
    services.AddMiddleware<AnotherMiddleware>("another-middleware");
}
```

## Creating Middlewares

Middlewares are the components that execute specific tasks in the pipeline. Each middleware should:

1. Implement the `IReleaseMiddleware` interface
2. Accept dependencies through constructor injection
3. Implement the `ExecuteAsync` method

```csharp
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins;

namespace MyCompany.Moonlit.Plugins.MyPlugin
{
    public class MyMiddleware : IReleaseMiddleware
    {
        private readonly ILogger<MyMiddleware> _logger;
        private readonly IMyService _myService;

        public MyMiddleware(ILogger<MyMiddleware> logger, IMyService myService)
        {
            _logger = logger;
            _myService = myService;
        }

        public async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
        {
            _logger.LogInformation("Executing MyMiddleware");

            // Get configuration from parameter
            var config = configuration.Get<MyMiddlewareConfig>();

            // Execute middleware logic
            var result = await _myService.DoSomethingAsync(config.SomeOption);

            // Return success with output
            return MiddlewareResult.Success(output =>
            {
                output.Add("result", result);
            });
        }
    }
}
```

### Release Context

The `ReleaseContext` provides access to:

- Configuration for the current step
- Output from previous steps
- Methods to add output for subsequent steps

```csharp
// Get configuration from parameter
var config = configuration.Get<MyMiddlewareConfig>();

// Get output from previous steps
var previousOutput = context.GetOutput<string>("previousStep", "outputName");

// Add output for subsequent steps
context.AddOutput("outputName", "outputValue");
```

### Middleware Result

The `MiddlewareResult` class represents the result of a middleware execution and provides information about its success or failure, along with any output or warnings:

```csharp
// Return success without output
return MiddlewareResult.Success();

// Return success with output
return MiddlewareResult.Success(output => 
{
    output.Add("key", "value");
});

// Return failure with error message
return MiddlewareResult.Failure("Something went wrong");

// Return success with a warning
return MiddlewareResult.Warning("This is a warning");

// Return success with a warning and output
return MiddlewareResult.Warning("This is a warning", output => 
{
    output.Add("key", "value");
});
```

## Configuration Classes

Configuration classes define the options for your plugin and middlewares. They should be simple POCO classes:

```csharp
namespace MyCompany.Moonlit.Plugins.MyPlugin
{
    public class MyPluginOptions
    {
        public string GlobalOption { get; set; }
    }

    public class MyMiddlewareConfig
    {
        public string SomeOption { get; set; }
        public int AnotherOption { get; set; }
    }
}
```

## Service Classes

Service classes provide reusable functionality that can be shared between middlewares:

```csharp
using System.Threading.Tasks;

namespace MyCompany.Moonlit.Plugins.MyPlugin
{
    public interface IMyService
    {
        Task<string> DoSomethingAsync(string input);
    }

    public class MyService : IMyService
    {
        public async Task<string> DoSomethingAsync(string input)
        {
            // Implement service logic
            return $"Processed: {input}";
        }
    }
}
```

## Packaging as a NuGet Package

To package your plugin as a NuGet package:

1. Add package metadata to your project file:

```xml
<PropertyGroup>
    <PackageId>MyCompany.Moonlit.Plugins.MyPlugin</PackageId>
    <Version>1.0.0</Version>
    <Authors>Your Name</Authors>
    <Description>My custom plugin for Moonlit</Description>
</PropertyGroup>
```

2. Create the NuGet package:

```bash
dotnet pack -c Release
```

3. Publish the package to a NuGet repository:

```bash
dotnet nuget push bin/Release/MyCompany.Moonlit.Plugins.MyPlugin.1.0.0.nupkg --source https://api.nuget.org/v3/index.json --api-key YOUR_API_KEY
```

## Using Your Plugin

To use your plugin in a Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "myplugin"
    url: "nuget://MyCompany.Moonlit.Plugins.MyPlugin/1.0.0"
    config:
      globalOption: "value"

stages:
  mystage:
    - name: mystep
      run: myplugin.my-middleware
      config:
        someOption: "value"
        anotherOption: 42
```

## Best Practices

- Keep your plugin focused on a specific domain or integration
- Use dependency injection for all services
- Implement proper error handling in your middlewares
- Add detailed logging to help diagnose issues
- Document your plugin's configuration options
- Write unit tests for your plugin
- Follow semantic versioning for your plugin releases

## Example: Complete Plugin

Here's a complete example of a simple plugin that provides a middleware to generate a random number:

```csharp
// RandomPluginStartup.cs
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins;

namespace MyCompany.Moonlit.Plugins.Random
{
    public sealed class RandomPluginStartup : PluginStartup
    {
        protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
        {
            services.AddSingleton<IRandomService, RandomService>();
        }

        protected override void AddMiddlewares(IServiceCollection services)
        {
            services.AddMiddleware<GenerateRandomNumberMiddleware>("generate");
        }
    }
}

// IRandomService.cs
using System;

namespace MyCompany.Moonlit.Plugins.Random
{
    public interface IRandomService
    {
        int GenerateNumber(int min, int max);
    }

    public class RandomService : IRandomService
    {
        private readonly System.Random _random = new System.Random();

        public int GenerateNumber(int min, int max)
        {
            return _random.Next(min, max + 1);
        }
    }
}

// GenerateRandomNumberMiddleware.cs
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins;

namespace MyCompany.Moonlit.Plugins.Random
{
    public class GenerateRandomNumberConfig
    {
        public int Min { get; set; } = 1;
        public int Max { get; set; } = 100;
    }

    public class GenerateRandomNumberMiddleware : IReleaseMiddleware
    {
        private readonly ILogger<GenerateRandomNumberMiddleware> _logger;
        private readonly IRandomService _randomService;

        public GenerateRandomNumberMiddleware(
            ILogger<GenerateRandomNumberMiddleware> logger,
            IRandomService randomService)
        {
            _logger = logger;
            _randomService = randomService;
        }

        public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
        {
            _logger.LogInformation("Generating random number");

            var config = configuration.Get<GenerateRandomNumberConfig>();

            var number = _randomService.GenerateNumber(config.Min, config.Max);

            _logger.LogInformation($"Generated random number: {number}");

            return Task.FromResult(MiddlewareResult.Success(output =>
            {
                output.Add("number", number);
            }));
        }
    }
}
```

## Next Steps

- Learn about [Moonlit's architecture](../guide/concepts/how-it-works.md)
- Explore the [configuration file structure](./config-file.md)
- See [examples](../plugins/examples/nuget-release.md) of complete pipelines
