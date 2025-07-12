---
title: Core API Reference
description: Detailed reference for Moonlit's core API classes and interfaces
---

# Core API Reference

This page provides a detailed reference for Moonlit's core API classes and interfaces. These components form the foundation of Moonlit's architecture and are essential for understanding how the system works.

## Core Components

### ReleaseContext

The `ReleaseContext` class is passed through the pipeline and contains information about the release process:

```csharp
public sealed record ReleaseContext
{
  public CancellationToken CancellationToken { get; init; }

  public string WorkingDirectory { get; init; } = Environment.CurrentDirectory;
}
```

#### Properties

| Property | Type | Description |
|----------|------|-------------|
| `CancellationToken` | `CancellationToken` | Used to check if the operation has been canceled |
| `WorkingDirectory` | `string` | The directory where the release process is running (defaults to the current directory) |

#### Extension Methods

The `ReleaseContext` class has several extension methods for working with configuration and output:

| Method | Description |
|--------|-------------|
| `GetOutput<T>(string stepName, string outputName)` | Gets output from a previous step |
| `AddOutput(string outputName, object value)` | Adds output for subsequent steps |
| `GetConfig<T>()` | Gets the configuration for the current step |

### MiddlewareResult

The `MiddlewareResult` class represents the result of a middleware execution:

```csharp
public sealed record MiddlewareResult
{
  public bool IsSuccessful { get; init; }

  public string? ErrorMessage { get; init; }

  public MiddlewareOutput Output { get; init; } = new();

  public List<string> Warnings { get; init; } = [];

  public static MiddlewareResult Success(Action<MiddlewareOutput>? setOutput = null)
  {
    var result = new MiddlewareResult {IsSuccessful = true};
    setOutput?.Invoke(result.Output);
    return result;
  }

  public static MiddlewareResult Failure(string errorMessage) =>
    new() {IsSuccessful = false, ErrorMessage = errorMessage};

  public static MiddlewareResult Warning(string warning, Action<MiddlewareOutput>? setOutput = null)
  {
    var result = new MiddlewareResult {IsSuccessful = true, Warnings = [warning]};
    setOutput?.Invoke(result.Output);
    return result;
  }
}
```

#### Properties

| Property | Type | Description |
|----------|------|-------------|
| `IsSuccessful` | `bool` | Indicates whether the middleware execution was successful |
| `ErrorMessage` | `string?` | Contains an error message if the execution failed |
| `Output` | `MiddlewareOutput` | Contains the output data produced by the middleware |
| `Warnings` | `List<string>` | Contains any warnings generated during execution |

#### Static Methods

| Method | Description |
|--------|-------------|
| `Success(Action<MiddlewareOutput>? setOutput = null)` | Creates a success result, optionally with output |
| `Failure(string errorMessage)` | Creates a failure result with an error message |
| `Warning(string warning, Action<MiddlewareOutput>? setOutput = null)` | Creates a success result with a warning, optionally with output |

### MiddlewareOutput

The `MiddlewareOutput` class is used to store output data from a middleware execution:

```csharp
public sealed class MiddlewareOutput
{
  private readonly Dictionary<string, object?> _data;

  public MiddlewareOutput(IReadOnlyDictionary<string, object?>? initialData = null)
  {
    this._data = new Dictionary<string, object?>(initialData ?? new Dictionary<string, object?>());
  }

  public void Add<T>(string key, T value)
  {
    ArgumentNullException.ThrowIfNull(key, nameof(key));

    if (this._data.ContainsKey(key))
    {
      throw new ArgumentException($"Key '{key}' already exists in the middleware output.",
        nameof(key));
    }

    if (value is string strValue)
    {
      this._data[key] = strValue;
      return;
    }

    this._data[key] = value;
  }

  public Dictionary<string, object?> ToDictionary(string scope)
  {
    return this._data.ToDictionary(
      kvp => $"output:{scope}:{kvp.Key}",
      kvp => kvp.Value
    );
  }
}
```

#### Methods

| Method | Description |
|--------|-------------|
| `Add<T>(string key, T value)` | Adds a key-value pair to the output data |
| `ToDictionary(string scope)` | Converts the output data to a dictionary with a specific scope |

### IReleaseMiddleware

The `IReleaseMiddleware` interface is implemented by all middlewares in Moonlit:

```csharp
public interface IReleaseMiddleware
{
  Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration);
}
```

#### Methods

| Method | Description |
|--------|-------------|
| `ExecuteAsync(ReleaseContext context, IConfiguration configuration)` | Executes the middleware with the given context and configuration |

### ReleaseMiddleware<TConfiguration>

The `ReleaseMiddleware<TConfiguration>` abstract class provides a more convenient way to handle configuration:

```csharp
public abstract class ReleaseMiddleware<TConfiguration> : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.GetRequired<TConfiguration>();
    return ExecuteAsync(context, config);
  }

  protected abstract Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, TConfiguration configuration);
}
```

#### Methods

| Method | Description |
|--------|-------------|
| `ExecuteAsync(ReleaseContext context, IConfiguration configuration)` | Implements the `IReleaseMiddleware` interface |
| `ExecuteAsync(ReleaseContext context, TConfiguration configuration)` | Abstract method to be implemented by derived classes |

### IPluginStartup

The `IPluginStartup` interface is implemented by all plugin startup classes:

```csharp
public interface IPluginStartup
{
  void Configure(IServiceCollection services, IConfiguration configuration);
}
```

#### Methods

| Method | Description |
|--------|-------------|
| `Configure(IServiceCollection services, IConfiguration configuration)` | Configures the plugin's services and middlewares |

### PluginStartup

The `PluginStartup` abstract class provides a more structured way to configure plugins:

```csharp
public abstract class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(services);
    ArgumentNullException.ThrowIfNull(configuration);

    this.ConfigurePlugin(services, configuration);
    this.AddMiddlewares(services);
  }

  protected virtual void ConfigurePlugin(IServiceCollection services, IConfiguration configuration) { }

  protected abstract void AddMiddlewares(IServiceCollection services);
}
```

#### Methods

| Method | Description |
|--------|-------------|
| `Configure(IServiceCollection services, IConfiguration configuration)` | Implements the `IPluginStartup` interface |
| `ConfigurePlugin(IServiceCollection services, IConfiguration configuration)` | Virtual method for configuring plugin-specific services |
| `AddMiddlewares(IServiceCollection services)` | Abstract method for registering middlewares |

## Extension Methods

### IServiceCollection Extensions

The `IServiceCollection` interface has several extension methods for registering middlewares:

| Method | Description |
|--------|-------------|
| `AddMiddleware<TMiddleware>(string name)` | Registers a middleware with a specific name |

### IConfiguration Extensions

The `IConfiguration` interface has several extension methods for working with configuration:

| Method | Description |
|--------|-------------|
| `Get<T>()` | Gets a configuration section and binds it to a strongly-typed object |
| `GetRequired<T>()` | Gets a required configuration section and binds it to a strongly-typed object |

## Next Steps

- Learn about [plugin development](./plugin-development.md)
- Explore the [configuration file reference](./config-file.md)
- See the [CLI reference](./cli.md) for command-line options