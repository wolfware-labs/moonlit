---
title: NodeJs Plugin
description: Documentation for the NodeJs plugin in Moonlit
---

# NodeJs Plugin

The NodeJs plugin provides comprehensive integration with Node.js and NPM. It allows you to run any scripts defined in package.json, build Node.js projects without packing them, pack and publish NPM packages to registries, manage package versions, and execute any Node.js-related operations.

## Installation

To use the NodeJs plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "npm"
    url: "nuget://Wolfware.Moonlit.Plugins.NodeJs/1.0.0"
    config:
      token: $(NPM_TOKEN)
```

Note that the NodeJs plugin requires a token to authenticate with NPM registries for publishing packages. You can set this token as an environment variable and reference it in your configuration file.

## Middlewares

The NodeJs plugin provides the following middlewares:

### run-script

The `run-script` middleware executes a script defined in the package.json file.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| directory | string | Yes | - | The directory containing the package.json file |
| script | string | Yes | - | The name of the script to run (as defined in package.json) |
| args | string | No | - | Additional arguments to pass to the script |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| output | string | The standard output from the script execution |

#### Example

```yaml
stages:
  build:
    - name: test
      run: npm.run-script
      config:
        directory: "./"
        script: "test"
    - name: build
      run: npm.run-script
      config:
        directory: "./"
        script: "build"
        args: "--production"
```

### build

The `build` middleware builds a Node.js project without packing it.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| directory | string | Yes | - | The directory containing the package.json file |
| command | string | No | "build" | The build command to run (typically a script in package.json) |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| output | string | The standard output from the build process |

#### Example

```yaml
stages:
  build:
    - name: build
      run: npm.build
      config:
        directory: "./"
        command: "build:prod"
```

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

### install

The `install` middleware installs dependencies for a Node.js project.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| directory | string | Yes | - | The directory containing the package.json file |
| production | boolean | No | false | Whether to install only production dependencies |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| output | string | The standard output from the installation process |

#### Example

```yaml
stages:
  setup:
    - name: install
      run: npm.install
      config:
        directory: "./"
        production: true
```

## Usage in Pipelines

The NodeJs plugin is commonly used in release pipelines for various Node.js-related tasks:

1. Setting up the environment by installing dependencies
2. Running tests, linting, and other quality checks
3. Building Node.js applications or libraries
4. Running custom scripts defined in package.json
5. Packing and publishing NPM packages with the correct version number
6. Executing any Node.js or NPM command as part of your pipeline

### Example: Complete Node.js Build and Release Pipeline

```yaml
stages:
  setup:
    - name: install
      run: npm.install
      config:
        directory: "./my-node-project"

  test:
    - name: lint
      run: npm.run-script
      config:
        directory: "./my-node-project"
        script: "lint"

    - name: test
      run: npm.run-script
      config:
        directory: "./my-node-project"
        script: "test"

  build:
    - name: build
      run: npm.build
      config:
        directory: "./my-node-project"
        command: "build:prod"

  publish:
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)

    - name: pack
      run: npm.pack
      config:
        directory: "./my-node-project"
        version: $(output:version:nextVersion)

    - name: publish
      run: npm.publish
      config:
        package: $(output:pack:packagePath)
        registry: "https://registry.npmjs.org"
```

These middlewares can be combined with other plugins like Git, GitHub, and Semantic Release to create a complete CI/CD pipeline for your Node.js projects.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [GitHub Plugin](./github.md) for GitHub API integration
- See the [Semantic Release Plugin](./semantic-release.md) for semantic versioning
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins
