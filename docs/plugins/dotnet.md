---
title: Dotnet Plugin
description: Documentation for the Dotnet plugin in Moonlit
---

# Dotnet Plugin

The Dotnet plugin provides integration with .NET projects. It allows you to build .NET projects without packing them, pack .NET projects into NuGet packages, and publish them to NuGet repositories.

## Installation

To use the Dotnet plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "dotnet"
    url: "nuget://Wolfware.Moonlit.Plugins.Dotnet/1.0.0"
    config:
      apiKey: $(NUGET_API_KEY)
```

Note that the Dotnet plugin requires an API key to authenticate with NuGet repositories for publishing packages. You can set this key as an environment variable and reference it in your configuration file.

## Middlewares

The Dotnet plugin provides the following middlewares:

### build

The `build` middleware builds a .NET project without packing it.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| project | string | Yes | - | The path to the .NET project file to build |
| configuration | string | No | "Release" | The configuration to use for building |

#### Outputs

This middleware does not produce any outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        configuration: "Release"
```

### pack

The `pack` middleware packs a .NET project into a NuGet package.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| project | string | Yes | - | The path to the .NET project file to pack |
| version | string | No | - | The version to use for the package |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| packagePath | string | The path to the created NuGet package file |

#### Example

```yaml
stages:
  publish:
    - name: pack
      run: dotnet.pack
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextVersion)
    - name: nextStep
      run: some.other-middleware
      config:
        packageFile: $(output:pack:packagePath)
```

### push

The `push` middleware pushes a NuGet package to a NuGet repository.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| package | string | Yes | - | The path to the NuGet package file to push |
| source | string | No | "https://api.nuget.org/v3/index.json" | The URL of the NuGet repository to push to |

#### Outputs

This middleware does not produce any outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  publish:
    - name: pack
      run: dotnet.pack
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextVersion)
    - name: push
      run: dotnet.push
      config:
        package: $(output:pack:packagePath)
        source: "https://api.nuget.org/v3/index.json"
```

## Usage in Pipelines

The Dotnet plugin is commonly used in release pipelines to:

1. Build .NET projects without packing them
2. Pack .NET projects into NuGet packages with the correct version number
3. Publish packages to NuGet repositories like NuGet.org or private feeds

These middlewares are typically used after testing the project, and after determining the version number using the Semantic Release plugin.

For a complete example of using the Dotnet plugin in a pipeline, see the [NuGet Release Pipeline](./examples/nuget-release.md) example.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [GitHub Plugin](./github.md) for GitHub API integration
- See the [Semantic Release Plugin](./semantic-release.md) for semantic versioning
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins