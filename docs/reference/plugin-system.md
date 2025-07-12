---
title: Plugin System Architecture
description: Detailed reference for Moonlit's plugin system architecture
---

# Plugin System Architecture

This page provides a detailed reference for Moonlit's plugin system architecture. It explains how plugins are structured, loaded, and integrated into the Moonlit pipeline.

## Plugin Structure

A Moonlit plugin is a NuGet package that contains:

1. **Startup Class**: Implements `IPluginStartup` or inherits from `PluginStartup`
2. **Middleware Classes**: Implement specific functionality for pipeline steps
3. **Service Classes**: Provide shared functionality
4. **Configuration Classes**: Define the configuration options for the plugin

### Plugin Package Structure

A typical plugin package has the following structure:

```
MyCompany.Moonlit.Plugins.MyPlugin.nupkg
├── lib
│   └── net6.0
│       └── MyCompany.Moonlit.Plugins.MyPlugin.dll
├── MyCompany.Moonlit.Plugins.MyPlugin.nuspec
└── [Content_Files]
```

The main assembly (`MyCompany.Moonlit.Plugins.MyPlugin.dll`) contains all the code for the plugin, including the startup class, middlewares, services, and configuration classes.

## Plugin Loading Process

When Moonlit starts, it follows these steps to load plugins:

1. **Parse Plugin Definitions**: Read the plugin entries from the configuration file
2. **Resolve NuGet Packages**: Download and extract the specified NuGet packages
3. **Load Assemblies**: Load the plugin assemblies into the application domain
4. **Find Startup Classes**: Locate classes that implement `IPluginStartup`
5. **Initialize Plugins**: Call the startup methods to register services and middlewares
6. **Configure Plugins**: Apply the configuration from the YAML file

### Plugin Resolution

Plugins are resolved using the NuGet package manager. The plugin URL in the configuration file specifies the package ID and version:

```yaml
plugins:
  - name: "myplugin"
    url: "nuget://MyCompany.Moonlit.Plugins.MyPlugin/1.0.0"
```

Moonlit uses the NuGet client API to download and extract the package from the configured NuGet sources.

### Assembly Loading

Once the NuGet package is downloaded and extracted, Moonlit loads the plugin assembly into the application domain using reflection:

```csharp
var assembly = Assembly.LoadFrom(assemblyPath);
```

### Startup Class Discovery

Moonlit uses reflection to find classes that implement the `IPluginStartup` interface:

```csharp
var startupTypes = assembly.GetTypes()
    .Where(t => typeof(IPluginStartup).IsAssignableFrom(t) && !t.IsAbstract)
    .ToList();
```

If multiple startup classes are found in a single assembly, Moonlit will use the first one.

### Plugin Initialization

Moonlit creates an instance of the startup class and calls its `Configure` method:

```csharp
var startup = (IPluginStartup)Activator.CreateInstance(startupType);
startup.Configure(services, configuration);
```

The `Configure` method is responsible for registering the plugin's services and middlewares with the dependency injection container.

## Plugin Registration

Plugins are registered in the Moonlit configuration file under the `plugins` section:

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

This configuration is passed to the plugin's startup class during initialization.

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

This configuration is passed to the middleware when it's executed.

## Middleware Registration

Middlewares are registered in the plugin's startup class:

```csharp
protected override void AddMiddlewares(IServiceCollection services)
{
    services.AddMiddleware<GetLatestTag>("latest-tag");
    services.AddMiddleware<GetItemsSinceCommit>("items-since-commit");
    services.AddMiddleware<CreateRelease>("create-release");
}
```

The `AddMiddleware` extension method registers a middleware with a specific name. This name is used to reference the middleware in the pipeline configuration.

## Middleware Execution

When a step in the pipeline is executed, Moonlit:

1. **Resolves the Middleware**: Looks up the middleware by name
2. **Creates an Instance**: Uses the dependency injection container to create an instance of the middleware
3. **Prepares the Context**: Creates a context object with the current state
4. **Executes the Middleware**: Calls the middleware's `ExecuteAsync` method
5. **Processes the Result**: Handles success, failure, or warnings

### Middleware Resolution

Middlewares are resolved using the format `pluginName.middlewareName`:

```yaml
run: gh.latest-tag
```

Moonlit looks up the plugin by name (`gh`) and then looks up the middleware by name (`latest-tag`) within that plugin.

### Middleware Instantiation

Middlewares are instantiated using the dependency injection container:

```csharp
var middleware = serviceProvider.GetRequiredService<TMiddleware>();
```

This allows middlewares to receive dependencies through constructor injection.

### Context Preparation

Before executing a middleware, Moonlit prepares a context object with the current state:

```csharp
var context = new ReleaseContext
{
    CancellationToken = cancellationToken,
    WorkingDirectory = workingDirectory
};
```

### Middleware Execution

Moonlit calls the middleware's `ExecuteAsync` method with the context and configuration:

```csharp
var result = await middleware.ExecuteAsync(context, configuration);
```

### Result Processing

Moonlit processes the result of the middleware execution:

```csharp
if (result.IsSuccessful)
{
    // Process output
    foreach (var output in result.Output.ToDictionary(stepName))
    {
        outputs[output.Key] = output.Value;
    }

    // Process warnings
    foreach (var warning in result.Warnings)
    {
        logger.LogWarning(warning);
    }
}
else
{
    // Handle failure
    logger.LogError(result.ErrorMessage);
    
    if (!continueOnError)
    {
        throw new PipelineExecutionException(result.ErrorMessage);
    }
}
```

## Plugin Dependencies

Plugins can depend on other plugins or external libraries. These dependencies are specified in the plugin's NuGet package:

```xml
<dependencies>
  <group targetFramework="net6.0">
    <dependency id="Wolfware.Moonlit.Plugins" version="1.0.0" />
    <dependency id="Octokit" version="0.50.0" />
  </group>
</dependencies>
```

When Moonlit loads a plugin, it also loads all of its dependencies.

## Plugin Isolation

Plugins are loaded into the same application domain as Moonlit, but they are isolated in terms of configuration and services:

1. **Configuration Isolation**: Each plugin has its own configuration section
2. **Service Isolation**: Services registered by a plugin are only available to that plugin

However, plugins can share data through the pipeline context.

## Best Practices

### Plugin Naming

Follow a consistent naming convention for your plugins:

- **Package ID**: `MyCompany.Moonlit.Plugins.MyPlugin`
- **Assembly Name**: `MyCompany.Moonlit.Plugins.MyPlugin`
- **Namespace**: `MyCompany.Moonlit.Plugins.MyPlugin`

### Plugin Versioning

Use semantic versioning for your plugins:

- **Major Version**: Breaking changes
- **Minor Version**: New features, non-breaking changes
- **Patch Version**: Bug fixes

### Plugin Documentation

Document your plugin thoroughly:

- **README**: Provide an overview of the plugin
- **XML Comments**: Add XML documentation comments to your code
- **Examples**: Include examples of how to use the plugin

## Next Steps

- Learn about [plugin development](./plugin-development.md)
- Explore the [core API reference](./core-api.md)
- See the [configuration file reference](./config-file.md)