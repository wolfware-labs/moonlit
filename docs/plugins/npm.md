---
title: NPM Plugin
description: Documentation for the NPM plugin in Moonlit
---

# NPM Plugin

The NPM plugin provides integration with NPM package management. It allows you to build and pack NPM packages, publish them to NPM registries, and manage package versions.

## Installation

To use the NPM plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "npm"
    url: "nuget://Wolfware.Moonlit.Plugins.Npm/1.0.0"
    config:
      token: $(NPM_TOKEN)
```

Note that the NPM plugin requires a token to authenticate with NPM registries for publishing packages. You can set this token as an environment variable and reference it in your configuration file.

## Middlewares

The NPM plugin provides the following middlewares:

### pack

The `pack` middleware builds and packs an NPM package.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| directory | string | Yes | - | The directory containing the package.json file |
| version | string | No | - | The version to use for the package |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| packagePath | string | The path to the created NPM package file |

#### Example

```yaml
stages:
  publish:
    - name: pack
      run: npm.pack
      config:
        directory: "./"
        version: $(output:version:nextVersion)
    - name: nextStep
      run: some.other-middleware
      config:
        packageFile: $(output:pack:packagePath)
```

### publish

The `publish` middleware publishes an NPM package to a registry.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| package | string | Yes | - | The path to the NPM package file to publish |
| registry | string | No | "https://registry.npmjs.org" | The URL of the NPM registry to publish to |

#### Outputs

This middleware does not produce any documented outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  publish:
    - name: pack
      run: npm.pack
      config:
        directory: "./"
        version: $(output:version:nextVersion)
    - name: publish
      run: npm.publish
      config:
        package: $(output:pack:packagePath)
        registry: "https://registry.npmjs.org"
```

## Usage in Pipelines

The NPM plugin is commonly used in release pipelines to:

1. Build and pack NPM packages with the correct version number
2. Publish packages to NPM registries like npmjs.org or private registries

These middlewares are typically used after determining the version number using the Semantic Release plugin.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [GitHub Plugin](./github.md) for GitHub API integration
- See the [Semantic Release Plugin](./semantic-release.md) for semantic versioning
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins