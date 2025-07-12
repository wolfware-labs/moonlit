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
    public class MyPluginStartup : PluginStartup
    {
        public override void ConfigureServices(IServiceCollection services, IConfiguration configuration)
        {
            // Register services
            services.AddSingleton<IMyService, MyService>();
            
            // Register middlewares
            services.AddTransient<MyMiddleware>();
        }
        
        public override void ConfigureMiddlewares(IMiddlewareRegistry registry)
        {
            // Register middlewares with names
            registry.Register("my-middleware", typeof(MyMiddleware));
        }
    }
}
```

### ConfigureServices Method

The `ConfigureServices` method is where you register your services and middlewares with the dependency injection container. This method is called when the plugin is loaded.

```csharp
public override void ConfigureServices(IServiceCollection services, IConfiguration configuration)
{
    // Register services
    services.AddSingleton<IMyService, MyService>();
    
    // Register middlewares
    services.AddTransient<MyMiddleware>();
    
    // Bind configuration
    services.Configure<MyPluginOptions>(configuration);
}
```

### ConfigureMiddlewares Method

The `ConfigureMiddlewares` method is where you register your middlewares with the middleware registry. This method is called after `ConfigureServices`.

```csharp
public override void ConfigureMiddlewares(IMiddlewareRegistry registry)
{
    // Register middlewares with names
    registry.Register("my-middleware", typeof(MyMiddleware));
    registry.Register("another-middleware", typeof(AnotherMiddleware));
}
```

## Creating Middlewares

Middlewares are the components that execute specific tasks in the pipeline. Each middleware should:

1. Implement the `IMiddleware` interface
2. Accept dependencies through constructor injection
3. Implement the `ExecuteAsync` method

```csharp
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins;

namespace MyCompany.Moonlit.Plugins.MyPlugin
{
    public class MyMiddleware : IMiddleware
    {
        private readonly ILogger<MyMiddleware> _logger;
        private readonly IMyService _myService;
        
        public MyMiddleware(ILogger<MyMiddleware> logger, IMyService myService)
        {
            _logger = logger;
            _myService = myService;
        }
        
        public async Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
        {
            _logger.LogInformation("Executing MyMiddleware");
            
            // Get configuration from context
            var config = context.GetConfig<MyMiddlewareConfig>();
            
            // Execute middleware logic
            var result = await _myService.DoSomethingAsync(config.SomeOption);
            
            // Add output to context
            context.AddOutput("result", result);
            
            // Return success
            return MiddlewareResult.Success();
        }
    }
}
```

### Middleware Context

The `MiddlewareContext` provides access to:

- Configuration for the current step
- Output from previous steps
- Methods to add output for subsequent steps

```csharp
// Get configuration
var config = context.GetConfig<MyMiddlewareConfig>();

// Get output from previous steps
var previousOutput = context.GetOutput<string>("previousStep", "outputName");

// Add output for subsequent steps
context.AddOutput("outputName", "outputValue");
```

### Middleware Result

The `MiddlewareResult` indicates the success or failure of the middleware execution:

```csharp
// Return success
return MiddlewareResult.Success();

// Return failure with error message
return MiddlewareResult.Failure("Something went wrong");

// Return failure with exception
return MiddlewareResult.Failure(exception);
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
    public class RandomPluginStartup : PluginStartup
    {
        public override void ConfigureServices(IServiceCollection services, IConfiguration configuration)
        {
            services.AddSingleton<IRandomService, RandomService>();
            services.AddTransient<GenerateRandomNumberMiddleware>();
        }
        
        public override void ConfigureMiddlewares(IMiddlewareRegistry registry)
        {
            registry.Register("generate", typeof(GenerateRandomNumberMiddleware));
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
    
    public class GenerateRandomNumberMiddleware : IMiddleware
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
        
        public Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
        {
            _logger.LogInformation("Generating random number");
            
            var config = context.GetConfig<GenerateRandomNumberConfig>();
            
            var number = _randomService.GenerateNumber(config.Min, config.Max);
            
            _logger.LogInformation($"Generated random number: {number}");
            
            context.AddOutput("number", number);
            
            return Task.FromResult(MiddlewareResult.Success());
        }
    }
}
```

## Next Steps

- Learn about [Moonlit's architecture](../guide/concepts/how-it-works.md)
- Explore the [configuration file structure](./config-file.md)
- See [examples](../plugins/examples/nuget-release.md) of complete pipelines