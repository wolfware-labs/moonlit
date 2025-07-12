---
title: Creating Custom Plugins
description: Learn how to create your own custom plugins for Moonlit
---

# Creating Custom Plugins

This guide will walk you through the process of creating a custom plugin for Moonlit. We'll create a simple plugin that provides a middleware to generate a random number.

## Prerequisites

Before you begin, make sure you have:

- .NET SDK 6.0 or later installed
- Basic knowledge of C# and .NET
- Moonlit CLI installed
- A code editor or IDE (Visual Studio, VS Code, etc.)

## Step 1: Create a New Project

First, create a new .NET Class Library project:

```bash
# Create a new directory for your plugin
mkdir MyRandomPlugin
cd MyRandomPlugin

# Create a new .NET Class Library project
dotnet new classlib -n MyCompany.Moonlit.Plugins.Random
cd MyCompany.Moonlit.Plugins.Random
```

## Step 2: Add Required Dependencies

Add a reference to the Wolfware.Moonlit.Plugins package:

```bash
dotnet add package Wolfware.Moonlit.Plugins
```

## Step 3: Create the Plugin Startup Class

The startup class is the entry point for your plugin. Create a new file called `RandomPluginStartup.cs`:

```csharp
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins;

namespace MyCompany.Moonlit.Plugins.Random
{
    public sealed class RandomPluginStartup : PluginStartup
    {
        protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
        {
            // Register services
            services.AddSingleton<IRandomService, RandomService>();
        }

        protected override void AddMiddlewares(IServiceCollection services)
        {
            // Register middlewares with names
            services.AddMiddleware<GenerateRandomNumberMiddleware>("generate");
        }
    }
}
```

## Step 4: Create a Service

Create a service that will provide the core functionality of your plugin. Create a new file called `RandomService.cs`:

```csharp
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
```

## Step 5: Create a Middleware

Create a middleware that will be executed in the pipeline. Create a new file called `GenerateRandomNumberMiddleware.cs`:

```csharp
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

            // Get configuration from context
            var config = context.GetConfig<GenerateRandomNumberConfig>();

            // Generate a random number
            var number = _randomService.GenerateNumber(config.Min, config.Max);

            _logger.LogInformation($"Generated random number: {number}");

            // Return success with output
            return Task.FromResult(MiddlewareResult.Success(output =>
            {
                output.Add("number", number);
            }));
        }
    }
}
```

## Step 6: Configure the Project for NuGet Packaging

Update your project file (`MyCompany.Moonlit.Plugins.Random.csproj`) to include NuGet package metadata:

```xml
<Project Sdk="Microsoft.NET.Sdk">

  <PropertyGroup>
    <TargetFramework>net6.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>

    <!-- NuGet package metadata -->
    <PackageId>MyCompany.Moonlit.Plugins.Random</PackageId>
    <Version>1.0.0</Version>
    <Authors>Your Name</Authors>
    <Company>Your Company</Company>
    <Description>A Moonlit plugin for generating random numbers</Description>
    <PackageTags>moonlit;plugin;random</PackageTags>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Wolfware.Moonlit.Plugins" Version="1.0.0" />
  </ItemGroup>

</Project>
```

## Step 7: Build and Package the Plugin

Build and package your plugin:

```bash
# Build the project
dotnet build -c Release

# Create a NuGet package
dotnet pack -c Release
```

This will create a NuGet package in the `bin/Release` directory.

## Step 8: Publish the Plugin

You can publish your plugin to a NuGet repository:

```bash
dotnet nuget push bin/Release/MyCompany.Moonlit.Plugins.Random.1.0.0.nupkg --source https://api.nuget.org/v3/index.json --api-key YOUR_API_KEY
```

Or, for local testing, you can use a local NuGet repository or reference the package directly from the file system.

## Step 9: Use the Plugin in a Moonlit Pipeline

Now you can use your plugin in a Moonlit pipeline. Create a file called `moonlit.yml`:

```yaml
name: "Random Number Generator"

plugins:
  - name: "random"
    url: "nuget://MyCompany.Moonlit.Plugins.Random/1.0.0"

stages:
  generate:
    - name: randomNumber
      run: random.generate
      config:
        min: 1
        max: 10

    - name: displayNumber
      run: console.log
      config:
        message: "The random number is: $(output:randomNumber:number)"
```

## Step 10: Run the Pipeline

Run the pipeline:

```bash
moonlit -f moonlit.yml
```

You should see output similar to:

```
[INFO] Starting pipeline: Random Number Generator
[INFO] Loading plugins...
[INFO] Running stage: generate
[INFO] Step: randomNumber
[INFO] Generating random number
[INFO] Generated random number: 7
[INFO] Step: displayNumber
[INFO] The random number is: 7
[INFO] Pipeline completed successfully
```

## Advanced Plugin Development

### Adding Multiple Middlewares

You can add multiple middlewares to your plugin:

```csharp
protected override void AddMiddlewares(IServiceCollection services)
{
    services.AddMiddleware<GenerateRandomNumberMiddleware>("generate");
    services.AddMiddleware<RollDiceMiddleware>("roll-dice");
    services.AddMiddleware<FlipCoinMiddleware>("flip-coin");
}
```

### Handling Asynchronous Operations

If your middleware needs to perform asynchronous operations, use the `async/await` pattern:

```csharp
public async Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    // Get configuration from context
    var config = context.GetConfig<MyConfig>();

    // Perform async operation
    var result = await _myService.DoSomethingAsync(config.SomeOption);

    // Return success with output
    return MiddlewareResult.Success(output =>
    {
        output.Add("result", result);
    });
}
```

### Error Handling

Proper error handling is important in middlewares:

```csharp
public async Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    try
    {
        // Get configuration from context
        var config = context.GetConfig<MyConfig>();

        // Validate configuration
        if (config.SomeOption == null)
        {
            return MiddlewareResult.Failure("SomeOption is required");
        }

        // Perform operation
        var result = await _myService.DoSomethingAsync(config.SomeOption);

        // Return success with output
        return MiddlewareResult.Success(output =>
        {
            output.Add("result", result);
        });
    }
    catch (Exception ex)
    {
        _logger.LogError(ex, "Error executing middleware");
        return MiddlewareResult.Failure(ex.Message);
    }
}
```

### Plugin Configuration

You can add global configuration for your plugin:

```csharp
public class RandomPluginOptions
{
    public string DefaultMode { get; set; }
    public int DefaultMin { get; set; } = 1;
    public int DefaultMax { get; set; } = 100;
}

public sealed class RandomPluginStartup : PluginStartup
{
    protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
    {
        // Bind configuration
        services.Configure<RandomPluginOptions>(configuration);

        // Register services
        services.AddSingleton<IRandomService, RandomService>();
    }

    protected override void AddMiddlewares(IServiceCollection services)
    {
        // Register middlewares
        services.AddMiddleware<GenerateRandomNumberMiddleware>("generate");
    }
}
```

Then, in your middleware, you can access the global configuration:

```csharp
private readonly RandomPluginOptions _options;

public GenerateRandomNumberMiddleware(
    ILogger<GenerateRandomNumberMiddleware> logger,
    IRandomService randomService,
    IOptions<RandomPluginOptions> options)
{
    _logger = logger;
    _randomService = randomService;
    _options = options.Value;
}

public Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    // Get configuration from context
    var config = context.GetConfig<GenerateRandomNumberConfig>();

    // Use default values from global configuration if not specified
    var min = config.Min ?? _options.DefaultMin;
    var max = config.Max ?? _options.DefaultMax;

    // Generate a random number
    var number = _randomService.GenerateNumber(min, max);

    // Return success with output
    return Task.FromResult(MiddlewareResult.Success(output =>
    {
        output.Add("number", number);
    }));
}
```

## Best Practices

- **Keep it focused**: Each plugin should have a clear, focused purpose
- **Use dependency injection**: Inject services rather than creating them directly
- **Handle errors gracefully**: Provide clear error messages and handle exceptions properly
- **Document your plugin**: Include XML documentation comments and README files
- **Write tests**: Unit test your plugin to ensure it works correctly
- **Follow semantic versioning**: Use proper versioning for your plugin releases

## Next Steps

- Learn more about [plugin development](../../reference/plugin-development.md) in the reference documentation
- Explore the [Moonlit architecture](../concepts/how-it-works.md) to understand how plugins fit into the bigger picture
- See [examples](../../plugins/examples/nuget-release.md) of complete pipelines that use plugins
