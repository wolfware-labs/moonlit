---
title: Configuration File Reference
description: Detailed reference for Moonlit's YAML configuration file format
---

# Configuration File Reference

This page provides a detailed reference for Moonlit's YAML configuration file format. For a conceptual overview of configuration, see the [Configuration](../guide/concepts/configuration.md) page.

## File Structure

A Moonlit configuration file has the following structure:

```yaml
name: "Pipeline Name"

plugins:
  - name: "plugin1"
    url: "nuget://Package.Name/Version"
    config:
      # Plugin-specific configuration

stages:
  stage1:
    - name: "step1"
      run: "plugin1.middleware1"
      config:
        # Step-specific configuration
```

## Top-Level Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | The name of your pipeline |
| `plugins` | array | Yes | A list of plugins to use in your pipeline |
| `stages` | object | Yes | A dictionary of stages, each containing a list of steps |

## Plugin Properties

Each plugin entry in the `plugins` array has the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | A unique identifier for the plugin |
| `url` | string | Yes | The NuGet package URL |
| `config` | object | No | Global configuration for the plugin |

### Plugin URL Format

The `url` property uses the following format:

```
nuget://{PackageName}/{Version}
```

For example:
```
nuget://Wolfware.Moonlit.Plugins.Git/1.0.0
```

## Stage Properties

Each stage is a named entry in the `stages` object. The value is an array of steps.

## Step Properties

Each step within a stage has the following properties:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | A unique identifier for the step |
| `run` | string | Yes | The middleware to execute, in the format `pluginName.middlewareName` |
| `condition` | string | No | A condition that must be true for the step to execute |
| `continueOnError` | boolean | No | Whether to continue execution if the step fails (default: `false`) |
| `config` | object | No | Configuration settings for the middleware |

### Step Run Format

The `run` property uses the following format:

```
{pluginName}.{middlewareName}
```

For example:
```
git.repo-context
```

### Step Condition Format

The `condition` property is a string expression that is evaluated at runtime. It can use output variables from previous steps:

```yaml
condition: "$(output:repo:branch) == 'main'"
```

## Variable Substitution

Moonlit supports variable substitution in your configuration file using the following syntax:

### Environment Variables

```
$(VARIABLE_NAME)
```

For example:
```yaml
token: $(GITHUB_TOKEN)
```

### Environment Variables with Default Values

```
$(VARIABLE_NAME:defaultValue)
```

For example:
```yaml
configuration: $(BUILD_CONFIGURATION:Release)
```

### Output Variables

```
$(output:stepName:propertyName)
```

For example:
```yaml
branch: $(output:repo:branch)
```

## Complete Example

Here's a complete example of a configuration file:

```yaml
name: "NuGet Package Release"

plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
  - name: "dotnet"
    url: "nuget://Wolfware.Moonlit.Plugins.Dotnet/1.0.0"
    config:
      apiKey: $(NUGET_API_KEY)

stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
        prereleaseMappings:
          main: next
          develop: beta

  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        configuration: $(BUILD_CONFIGURATION:Release)

  publish:
    - name: pack
      run: dotnet.pack
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextVersion)

    - name: push
      run: dotnet.push
      condition: $(output:version:isPrerelease) == false
      config:
        package: $(output:pack:packagePath)
        source: "https://api.nuget.org/v3/index.json"
```

## Schema Validation

Moonlit validates your configuration file against a schema to ensure it's correctly formatted. Common validation errors include:

- Missing required properties
- Invalid property types
- Unknown properties
- Invalid variable syntax
- Circular dependencies in variable references

If your configuration file is invalid, Moonlit will display an error message with details about the validation failure.

## Best Practices

- Use meaningful names for plugins, stages, and steps
- Group related steps into stages
- Use environment variables for sensitive information
- Use output variables to pass data between steps
- Provide default values for environment variables when appropriate
- Keep your configuration file under version control
- Split large pipelines into multiple files and use includes (if supported)

## Next Steps

- Explore the [CLI Reference](./cli.md) for command-line options
- Learn about [creating custom plugins](../guide/advanced/custom-plugins.md)
- See [examples](../plugins/examples/nuget-release.md) of complete pipelines
