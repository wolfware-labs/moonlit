---
title: Middleware Pipeline
description: Learn how Moonlit's middleware pipeline works and how to create custom middlewares
---

# Middleware Pipeline

Moonlit uses a middleware pipeline pattern to execute steps in a release pipeline. This page explains how the middleware pipeline works and how to create custom middlewares.

## What is a Middleware Pipeline?

A middleware pipeline is a series of components (middlewares) that process a request in sequence. Each middleware:

1. Receives a context object
2. Performs its specific task
3. Updates the context with its results
4. Passes the context to the next middleware in the pipeline

This pattern is commonly used in web frameworks like ASP.NET Core, but Moonlit adapts it for release pipeline automation.

## Middleware Pipeline in Moonlit

In Moonlit, the middleware pipeline is used to execute steps in a release pipeline:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│                 │     │                 │     │                 │
│  Middleware 1   │────▶│  Middleware 2   │────▶│  Middleware 3   │────▶ ...
│                 │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
       │                       │                       │
       │                       │                       │
       ▼                       ▼                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│                        Pipeline Context                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

Each middleware in the pipeline:

1. Receives the `MiddlewareContext` object
2. Performs its specific task (e.g., building a project, creating a release)
3. Updates the context with its results (e.g., build output, release URL)
4. Returns a `MiddlewareResult` indicating success or failure

## Middleware Interface

All middlewares in Moonlit implement the `IMiddleware` interface:

```csharp
public interface IMiddleware
{
    Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context);
}
```

The `ExecuteAsync` method is called when the middleware is executed in the pipeline. It receives a `MiddlewareContext` object and returns a `MiddlewareResult`.

## Middleware Context

The `MiddlewareContext` object is passed through the pipeline and contains:

- Configuration for the current step
- Output from previous steps
- Methods to add output for subsequent steps

### Getting Configuration

You can get the configuration for the current step from the context:

```csharp
public Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    // Get configuration from context
    var config = context.GetConfig<MyMiddlewareConfig>();
    
    // Use the configuration
    var option = config.SomeOption;
    
    return Task.FromResult(MiddlewareResult.Success());
}
```

The configuration is automatically deserialized from the YAML configuration file to your strongly-typed configuration class.

### Getting Output from Previous Steps

You can get output from previous steps:

```csharp
public Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    // Get output from a previous step
    var previousOutput = context.GetOutput<string>("previousStep", "outputName");
    
    // Use the output
    // ...
    
    return Task.FromResult(MiddlewareResult.Success());
}
```

The `GetOutput<T>` method takes two parameters:
- The name of the step that produced the output
- The name of the output property

### Adding Output for Subsequent Steps

You can add output for subsequent steps:

```csharp
public Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    // Perform some operation
    var result = "Hello, World!";
    
    // Add output to context
    context.AddOutput("outputName", result);
    
    return Task.FromResult(MiddlewareResult.Success());
}
```

The `AddOutput` method takes two parameters:
- The name of the output property
- The value of the output property

## Middleware Result

The `MiddlewareResult` indicates the success or failure of the middleware execution:

```csharp
// Return success
return MiddlewareResult.Success();

// Return failure with error message
return MiddlewareResult.Failure("Something went wrong");

// Return failure with exception
return MiddlewareResult.Failure(exception);
```

If a middleware returns a failure result, the pipeline execution stops by default, unless the step is configured to continue on error.

## Creating a Custom Middleware

To create a custom middleware, follow these steps:

1. Create a configuration class (optional):

```csharp
public class MyMiddlewareConfig
{
    public string SomeOption { get; set; }
    public int AnotherOption { get; set; }
}
```

2. Create a middleware class that implements `IMiddleware`:

```csharp
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
```

3. Register the middleware in your plugin's startup class:

```csharp
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
```

4. Use the middleware in your pipeline:

```yaml
stages:
  mystage:
    - name: mystep
      run: myplugin.my-middleware
      config:
        someOption: "value"
        anotherOption: 42
```

## Middleware Best Practices

### 1. Keep Middlewares Focused

Each middleware should have a single responsibility. If a middleware is doing too much, consider splitting it into multiple middlewares.

### 2. Use Dependency Injection

Use constructor injection to receive dependencies:

```csharp
public class MyMiddleware : IMiddleware
{
    private readonly ILogger<MyMiddleware> _logger;
    private readonly IMyService _myService;
    
    public MyMiddleware(ILogger<MyMiddleware> logger, IMyService myService)
    {
        _logger = logger;
        _myService = myService;
    }
    
    // ...
}
```

### 3. Handle Errors Gracefully

Provide clear error messages and handle exceptions properly:

```csharp
public async Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    try
    {
        // Get configuration from context
        var config = context.GetConfig<MyMiddlewareConfig>();
        
        // Validate configuration
        if (string.IsNullOrEmpty(config.SomeOption))
        {
            return MiddlewareResult.Failure("SomeOption is required");
        }
        
        // Execute middleware logic
        var result = await _myService.DoSomethingAsync(config.SomeOption);
        
        // Add output to context
        context.AddOutput("result", result);
        
        // Return success
        return MiddlewareResult.Success();
    }
    catch (Exception ex)
    {
        _logger.LogError(ex, "Error executing middleware");
        return MiddlewareResult.Failure(ex);
    }
}
```

### 4. Add Detailed Logging

Add detailed logging to help diagnose issues:

```csharp
public async Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    _logger.LogInformation("Starting execution of MyMiddleware");
    
    // Get configuration from context
    var config = context.GetConfig<MyMiddlewareConfig>();
    _logger.LogDebug("Configuration: {@Config}", config);
    
    // Execute middleware logic
    _logger.LogInformation("Executing service operation");
    var result = await _myService.DoSomethingAsync(config.SomeOption);
    _logger.LogDebug("Service operation result: {Result}", result);
    
    // Add output to context
    context.AddOutput("result", result);
    
    _logger.LogInformation("MyMiddleware execution completed successfully");
    return MiddlewareResult.Success();
}
```

### 5. Document Your Middleware

Document your middleware's purpose, configuration options, and outputs:

```csharp
/// <summary>
/// A middleware that does something useful.
/// </summary>
public class MyMiddleware : IMiddleware
{
    // ...
}

/// <summary>
/// Configuration options for MyMiddleware.
/// </summary>
public class MyMiddlewareConfig
{
    /// <summary>
    /// Some option that does something.
    /// </summary>
    public string SomeOption { get; set; }
    
    /// <summary>
    /// Another option that does something else.
    /// </summary>
    public int AnotherOption { get; set; }
}
```

## Advanced Middleware Techniques

### Conditional Execution

You can make a middleware execute conditionally based on the pipeline context:

```csharp
public Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
{
    // Get configuration from context
    var config = context.GetConfig<MyMiddlewareConfig>();
    
    // Get output from a previous step
    var branch = context.GetOutput<string>("repo", "branch");
    
    // Skip execution based on a condition
    if (branch != "main" && !config.RunOnAllBranches)
    {
        _logger.LogInformation("Skipping execution on branch {Branch}", branch);
        return Task.FromResult(MiddlewareResult.Success());
    }
    
    // Continue with normal execution
    // ...
    
    return Task.FromResult(MiddlewareResult.Success());
}
```

### Middleware Composition

You can compose multiple middlewares to create more complex behaviors:

```csharp
public class CompositeMiddleware : IMiddleware
{
    private readonly IMiddleware _firstMiddleware;
    private readonly IMiddleware _secondMiddleware;
    
    public CompositeMiddleware(IMiddleware firstMiddleware, IMiddleware secondMiddleware)
    {
        _firstMiddleware = firstMiddleware;
        _secondMiddleware = secondMiddleware;
    }
    
    public async Task<MiddlewareResult> ExecuteAsync(MiddlewareContext context)
    {
        // Execute the first middleware
        var firstResult = await _firstMiddleware.ExecuteAsync(context);
        if (!firstResult.IsSuccess)
        {
            return firstResult;
        }
        
        // Execute the second middleware
        return await _secondMiddleware.ExecuteAsync(context);
    }
}
```

## Next Steps

- Learn about [dependency injection](./dependency-injection.md) in Moonlit
- See how to [create custom plugins](./custom-plugins.md) that provide middlewares
- Explore the [configuration system](../concepts/configuration.md) to understand how to configure middlewares