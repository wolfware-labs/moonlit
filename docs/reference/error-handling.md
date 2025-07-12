---
title: Error Handling and Troubleshooting
description: Guide to error handling and troubleshooting in Moonlit
---

# Error Handling and Troubleshooting

This page provides information about error handling in Moonlit and how to troubleshoot common issues.

## Error Handling in Middlewares

Middlewares in Moonlit can handle errors in several ways:

### Returning Failure Results

The most common way to handle errors is to return a `MiddlewareResult.Failure` with an error message:

```csharp
public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    try
    {
        // Get configuration
        var config = configuration.Get<MyMiddlewareConfig>();

        // Validate configuration
        if (string.IsNullOrEmpty(config.SomeOption))
        {
            return Task.FromResult(MiddlewareResult.Failure("SomeOption is required"));
        }

        // Execute middleware logic
        // ...

        return Task.FromResult(MiddlewareResult.Success());
    }
    catch (Exception ex)
    {
        return Task.FromResult(MiddlewareResult.Failure(ex.Message));
    }
}
```

### Using Try-Catch Blocks

You can use try-catch blocks to handle exceptions and return appropriate failure results:

```csharp
public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    try
    {
        // Execute middleware logic that might throw exceptions
        // ...

        return Task.FromResult(MiddlewareResult.Success());
    }
    catch (SpecificException ex)
    {
        // Handle specific exception
        return Task.FromResult(MiddlewareResult.Failure($"Specific error: {ex.Message}"));
    }
    catch (Exception ex)
    {
        // Handle general exception
        return Task.FromResult(MiddlewareResult.Failure($"Unexpected error: {ex.Message}"));
    }
}
```

### Logging Errors

Always log errors to help with debugging:

```csharp
private readonly ILogger<MyMiddleware> _logger;

public MyMiddleware(ILogger<MyMiddleware> logger)
{
    _logger = logger;
}

public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    try
    {
        // Execute middleware logic
        // ...

        return Task.FromResult(MiddlewareResult.Success());
    }
    catch (Exception ex)
    {
        _logger.LogError(ex, "Error executing middleware");
        return Task.FromResult(MiddlewareResult.Failure(ex.Message));
    }
}
```

### Warnings vs. Errors

Sometimes you might want to issue a warning instead of failing the pipeline:

```csharp
public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    // Execute middleware logic
    // ...

    if (someCondition)
    {
        return Task.FromResult(MiddlewareResult.Warning("This is a warning message"));
    }

    return Task.FromResult(MiddlewareResult.Success());
}
```

## Pipeline Error Handling

### ContinueOnError

By default, if a middleware returns a failure result, the pipeline execution stops. However, you can configure a step to continue on error:

```yaml
stages:
  mystage:
    - name: mystep
      run: myplugin.my-middleware
      continueOnError: true
      config:
        # Step configuration
```

### Conditional Execution

You can use conditions to skip steps based on the results of previous steps:

```yaml
stages:
  mystage:
    - name: firststep
      run: myplugin.first-middleware
      
    - name: secondstep
      run: myplugin.second-middleware
      condition: $(output:firststep:success) == true
```

## Common Issues and Solutions

### Configuration Issues

#### Missing Configuration

**Issue**: A middleware fails with an error message about missing configuration.

**Solution**: Check your configuration file to ensure all required configuration properties are provided:

```yaml
stages:
  mystage:
    - name: mystep
      run: myplugin.my-middleware
      config:
        requiredOption: "value"  # Make sure this is provided
```

#### Invalid Configuration

**Issue**: A middleware fails with an error message about invalid configuration.

**Solution**: Check the documentation for the middleware to ensure you're providing valid configuration values.

### Plugin Issues

#### Plugin Not Found

**Issue**: Moonlit fails to load a plugin with an error message about the plugin not being found.

**Solution**: Check the plugin URL in your configuration file:

```yaml
plugins:
  - name: "myplugin"
    url: "nuget://Package.Name/Version"  # Make sure this is correct
```

#### Plugin Version Conflict

**Issue**: Moonlit fails to load a plugin with an error message about version conflicts.

**Solution**: Try specifying a different version of the plugin or check for compatibility issues with other plugins.

### Middleware Issues

#### Middleware Not Found

**Issue**: Moonlit fails to execute a step with an error message about the middleware not being found.

**Solution**: Check the middleware name in your configuration file:

```yaml
stages:
  mystage:
    - name: mystep
      run: myplugin.my-middleware  # Make sure this is correct
```

#### Middleware Execution Failure

**Issue**: A middleware fails during execution.

**Solution**: Check the error message and logs for details about the failure. Common causes include:

- Invalid configuration
- Missing dependencies
- External service failures
- File system permissions

## Logging and Debugging

### Enabling Verbose Logging

You can enable verbose logging to get more detailed information about what's happening:

```bash
moonlit -f pipeline.yml --verbose
```

### Log Files

Moonlit logs are written to the console by default, but you can redirect them to a file:

```bash
moonlit -f pipeline.yml > moonlit.log 2>&1
```

### Debugging Middlewares

If you're developing a custom middleware, you can add debug logging to help troubleshoot issues:

```csharp
public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
{
    _logger.LogDebug("Starting execution of MyMiddleware");
    _logger.LogDebug("Configuration: {@Config}", configuration);

    // Execute middleware logic
    // ...

    _logger.LogDebug("Completed execution of MyMiddleware");
    return Task.FromResult(MiddlewareResult.Success());
}
```

## Next Steps

- Learn about [middleware pipelines](../guide/advanced/middleware.md)
- Explore the [configuration file reference](./config-file.md)
- See the [CLI reference](./cli.md) for command-line options