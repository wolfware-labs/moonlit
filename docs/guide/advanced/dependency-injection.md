---
title: Dependency Injection in Moonlit
description: Learn how Moonlit uses dependency injection to manage services and share data
---

# Dependency Injection in Moonlit

Moonlit relies heavily on dependency injection (DI) to manage services and share data between components. This page explains how dependency injection works in Moonlit and how you can use it in your plugins and middlewares.

## What is Dependency Injection?

Dependency injection is a design pattern that allows a class to receive its dependencies from external sources rather than creating them itself. This promotes:

- **Loose coupling**: Classes don't need to know how to create their dependencies
- **Testability**: Dependencies can be easily mocked for testing
- **Flexibility**: Dependencies can be swapped out without changing the class that uses them
- **Reusability**: Services can be shared across multiple components

Moonlit uses the Microsoft.Extensions.DependencyInjection library for its dependency injection system, which is the same system used by ASP.NET Core.

## How Moonlit Uses Dependency Injection

Moonlit uses dependency injection at several levels:

1. **Core Services**: Moonlit registers its core services with the DI container
2. **Plugin Services**: Plugins register their services with the DI container
3. **Middlewares**: Middlewares receive their dependencies through constructor injection
4. **Configuration**: Configuration is bound to strongly-typed options classes

### DI Container Lifecycle

When Moonlit starts, it:

1. Creates a new `ServiceCollection`
2. Registers core services
3. Loads plugins and calls their `ConfigureServices` methods
4. Builds a `ServiceProvider` from the `ServiceCollection`
5. Uses the `ServiceProvider` to resolve services as needed

## Registering Services in Plugins

In your plugin's startup class, you can register services with the DI container:

```csharp
protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
{
    // Register a singleton service
    services.AddSingleton<IMyService, MyService>();

    // Register a transient service
    services.AddTransient<IMyTransientService, MyTransientService>();

    // Register a scoped service
    services.AddScoped<IMyScopedService, MyScopedService>();

    // Bind configuration
    services.Configure<MyPluginOptions>(configuration);
}

protected override void AddMiddlewares(IServiceCollection services)
{
    // Register middlewares
    services.AddMiddleware<MyMiddleware>("my-middleware");
}
```

### Service Lifetimes

When registering services, you need to specify their lifetime:

- **Singleton**: A single instance is created and shared throughout the application
- **Transient**: A new instance is created each time the service is requested
- **Scoped**: A new instance is created for each scope (in Moonlit, each pipeline execution is a scope)

Choose the appropriate lifetime based on your service's requirements:

- Use **Singleton** for stateless services or services that maintain global state
- Use **Transient** for lightweight, stateless services
- Use **Scoped** for services that need to maintain state for the duration of a pipeline execution

## Using Dependency Injection in Middlewares

Middlewares receive their dependencies through constructor injection:

```csharp
public class MyMiddleware : IReleaseMiddleware
{
    private readonly ILogger<MyMiddleware> _logger;
    private readonly IMyService _myService;
    private readonly IOptions<MyPluginOptions> _options;

    public MyMiddleware(
        ILogger<MyMiddleware> logger,
        IMyService myService,
        IOptions<MyPluginOptions> options)
    {
        _logger = logger;
        _myService = myService;
        _options = options;
    }

    public async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
    {
        _logger.LogInformation("Executing MyMiddleware");

        // Use the injected services
        var result = await _myService.DoSomethingAsync();

        // Use the options
        var option = _options.Value.SomeOption;

        // Return success
        return MiddlewareResult.Success();
    }
}
```

## Common Services Available for Injection

Moonlit provides several services that you can inject into your middlewares:

### Logging

```csharp
private readonly ILogger<MyMiddleware> _logger;

public MyMiddleware(ILogger<MyMiddleware> logger)
{
    _logger = logger;
}

public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    _logger.LogInformation("This is an information message");
    _logger.LogWarning("This is a warning message");
    _logger.LogError("This is an error message");

    return Task.FromResult(MiddlewareResult.Success());
}
```

### Configuration

```csharp
private readonly IOptions<MyPluginOptions> _options;

public MyMiddleware(IOptions<MyPluginOptions> options)
{
    _options = options;
}

public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    var option = _options.Value.SomeOption;

    return Task.FromResult(MiddlewareResult.Success());
}
```

### HttpClient Factory

```csharp
private readonly IHttpClientFactory _httpClientFactory;

public MyMiddleware(IHttpClientFactory httpClientFactory)
{
    _httpClientFactory = httpClientFactory;
}

public async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    var client = _httpClientFactory.CreateClient("MyClient");
    var response = await client.GetAsync("https://api.example.com");

    return MiddlewareResult.Success();
}
```

## Sharing Data Between Middlewares

There are several ways to share data between middlewares:

### 1. Using the Pipeline Context

The most common way to share data between middlewares is through the pipeline context:

```csharp
// In the first middleware
public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    // Return success with output
    return Task.FromResult(MiddlewareResult.Success(output =>
    {
        output.Add("myData", "Hello, World!");
    }));
}

// In a later middleware
public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    // Get data from the context
    var myData = context.GetOutput<string>("previousStep", "myData");

    return Task.FromResult(MiddlewareResult.Success());
}
```

### 2. Using Scoped Services

You can use scoped services to share data for the duration of a pipeline execution:

```csharp
// Define a scoped service
public interface IPipelineState
{
    string GetValue(string key);
    void SetValue(string key, string value);
}

public class PipelineState : IPipelineState
{
    private readonly Dictionary<string, string> _values = new Dictionary<string, string>();

    public string GetValue(string key)
    {
        return _values.TryGetValue(key, out var value) ? value : null;
    }

    public void SetValue(string key, string value)
    {
        _values[key] = value;
    }
}

// Register the service as scoped
protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
{
    services.AddScoped<IPipelineState, PipelineState>();
}

// In the first middleware
public class FirstMiddleware : IReleaseMiddleware
{
    private readonly IPipelineState _state;

    public FirstMiddleware(IPipelineState state)
    {
        _state = state;
    }

    public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
    {
        // Set data in the state
        _state.SetValue("myKey", "myValue");

        return Task.FromResult(MiddlewareResult.Success());
    }
}

// In a later middleware
public class SecondMiddleware : IReleaseMiddleware
{
    private readonly IPipelineState _state;

    public SecondMiddleware(IPipelineState state)
    {
        _state = state;
    }

    public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
    {
        // Get data from the state
        var value = _state.GetValue("myKey");

        return Task.FromResult(MiddlewareResult.Success());
    }
}
```

### 3. Using Singleton Services

For data that needs to persist across pipeline executions, you can use singleton services:

```csharp
// Define a singleton service
public interface IGlobalState
{
    string GetValue(string key);
    void SetValue(string key, string value);
}

public class GlobalState : IGlobalState
{
    private readonly Dictionary<string, string> _values = new Dictionary<string, string>();
    private readonly object _lock = new object();

    public string GetValue(string key)
    {
        lock (_lock)
        {
            return _values.TryGetValue(key, out var value) ? value : null;
        }
    }

    public void SetValue(string key, string value)
    {
        lock (_lock)
        {
            _values[key] = value;
        }
    }
}

// Register the service as singleton
protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
{
    services.AddSingleton<IGlobalState, GlobalState>();
}
```

## Best Practices

- **Use constructor injection**: Always use constructor injection to receive dependencies
- **Keep services focused**: Each service should have a single responsibility
- **Choose the right lifetime**: Use the appropriate lifetime for your services
- **Avoid service locator pattern**: Don't use `IServiceProvider` directly in your code
- **Use strongly-typed options**: Bind configuration to strongly-typed options classes
- **Prefer context for middleware communication**: Use the pipeline context for sharing data between middlewares
- **Document dependencies**: Document the dependencies of your middlewares and services

## Next Steps

- Learn about [middleware pipelines](./middleware.md) in Moonlit
- See how to [create custom plugins](./custom-plugins.md) that use dependency injection
- Explore the [configuration system](../concepts/configuration.md) to understand how configuration works with DI
