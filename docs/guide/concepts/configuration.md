---
title: Configuration
description: Learn how to configure Moonlit using YAML files and environment variables
---

# Configuration

Moonlit uses YAML configuration files to define your release pipeline. This page explains the structure of these files and how to use environment variables and output variables for dynamic configuration.

## Configuration File Structure

A Moonlit configuration file has the following structure:

```yaml
name: "My Pipeline"

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

### Top-Level Properties

- **name**: The name of your pipeline
- **plugins**: A list of plugins to use in your pipeline
- **stages**: A dictionary of stages, each containing a list of steps
- **variables**: A dictionary of variables that can be used throughout the pipeline
- **arguments**: A dictionary of arguments that can be used throughout the pipeline

## Plugin Configuration

Each plugin entry in the `plugins` section has the following properties:

```yaml
plugins:
  - name: "git"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Git/1.0.0-next.5"
    config:
      # Plugin-specific configuration
```

- **name**: A unique identifier for the plugin
- **url**: The NuGet package URL
- **config** (optional): Global configuration for the plugin

## Stage Configuration

Each stage is a named entry in the `stages` section:

```yaml
stages:
  build:
    # Steps for the build stage

  publish:
    # Steps for the publish stage
```

## Step Configuration

Each step within a stage has the following properties:

```yaml
- name: "build"
  run: "dotnet.build"
  condition: "$(output:repo:branch) == 'main'"
  continueOnError: false
  config:
    project: "./src/MyProject.csproj"
```

- **name**: A unique identifier for the step
- **run**: The middleware to execute, in the format `pluginName.middlewareName`
- **condition** (optional): A condition that must be true for the step to execute (corresponds to `ExecuteOn` in the API)
- **haltIf** (optional): A condition that, when true, will halt the pipeline execution after this step completes
- **continueOnError** (optional): Whether to continue execution if the step fails
- **config** (optional): Configuration settings for the middleware

## Variable Substitution

Moonlit supports variable substitution in your configuration file using the following syntax:

### Environment Variables

You can use environment variables in your configuration:

```yaml
config:
  token: $(GITHUB_TOKEN)
```

When Moonlit runs, it will replace `$(GITHUB_TOKEN)` with the value of the `GITHUB_TOKEN` environment variable.

### Output Variables

You can use output from previous steps:

```yaml
config:
  branch: $(output:repo:branch)
  version: $(output:version:nextVersion)
```

The syntax is `$(output:stepName:propertyName)`, where:
- `stepName` is the name of a previous step
- `propertyName` is the name of an output property from that step

### Default Values

You can provide default values for variables:

```yaml
config:
  configuration: $(BUILD_CONFIGURATION:Release)
```

If the `BUILD_CONFIGURATION` environment variable is not set, Moonlit will use `Release` as the default value.

## Variables and Arguments

Moonlit supports two special top-level sections for sharing data across your pipeline:

### Variables

The `variables` section defines a dictionary of values that can be used throughout your pipeline:

```yaml
variables:
  projectName: "MyProject"
  buildConfiguration: "Release"
  version: "1.0.0"
```

You can reference these variables in your configuration using the `$(vars:variableName)` syntax:

```yaml
config:
  project: "./src/$(vars:projectName).csproj"
  configuration: $(vars:buildConfiguration)
```

### Arguments

The `arguments` section defines a dictionary of values that can be overridden by command-line arguments:

```yaml
arguments:
  environment: "production"
  skipTests: false
```

You can reference these arguments in your configuration using the `$(args:argumentName)` syntax:

```yaml
config:
  environment: $(args:environment)
  runTests: $(args:skipTests) == false
```

Command-line arguments take precedence over arguments defined in the configuration file.

## Configuration Inheritance

Configuration values can be defined at multiple levels:

1. **Plugin-level configuration**: Defined in the `plugins` section
2. **Step-level configuration**: Defined in the `config` property of a step
3. **Variables**: Defined in the `variables` section
4. **Arguments**: Defined in the `arguments` section, can be overridden by command-line arguments

The precedence order (from highest to lowest) is:
1. Command-line arguments
2. Step-level configuration
3. Plugin-level configuration
4. Variables defined in the configuration file

## Example: Complete Configuration

Here's an example of a complete configuration file:

```yaml
name: "NuGet Package Release"

variables:
  projectPath: "./src/MyProject.csproj"
  nugetSource: "https://api.nuget.org/v3/index.json"
  versionPrefix: "v"

arguments:
  configuration: "Release"
  skipPush: false
  prerelease: false

plugins:
  - name: "git"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Git/1.0.0-next.5"
  - name: "gh"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Github/1.0.0-next.6"
    config:
      token: $(GITHUB_TOKEN)
  - name: "sr"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0-next.5"
    config:
      openAi:
        apiKey: $(OPENAI_API_KEY)
  - name: "dotnet"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Dotnet/1.0.0-next.5"
    config:
      nugetApiKey: $(NUGET_API_KEY)

stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: git.latest-tag
      config:
        prefix: $(vars:versionPrefix)
    - name: commits
      run: git.commits
    - name: conventionalCommits
      run: sr.analyze
      haltIf: output.conventionalCommits.commitCount == 0
      config:
        commits: $(output:commits:details)
    - name: version
      run: sr.calculate-version
      haltIf: "!output.version.hasNewVersion"
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
        prereleaseMappings:
          main: next
          develop: beta
    - name: changelog
      run: sr.generate-changelog

  build:
    - name: build
      run: dotnet.build
      config:
        project: $(vars:projectPath)
        version: $(output:version:nextFullVersion)
        configuration: $(args:configuration)
    - name: pack
      run: dotnet.pack
      config:
        project: $(vars:projectPath)
        version: $(output:version:nextFullVersion)

  release:
    - name: push
      run: dotnet.push
      condition: $(args:skipPush) == false && ($(args:prerelease) == true || $(output:version:isPrerelease) == false)
      config:
        package: $(output:pack:packagePath)
        source: $(vars:nugetSource)
    - name: createRelease
      run: gh.create-release
      config:
        name: "Release $(output:version:nextVersion)"
        tag: "$(vars:versionPrefix)$(output:version:nextVersion)"
        changelog: $(output:changelog:categories)
        prerelease: $(output:version:isPrerelease)
```

## Using the Configuration File

You can specify the configuration file to use with the `-f` or `--file` option:

```bash
moonlit -f ./path/to/moonlit.yml
```

If you don't specify a file, Moonlit will look for a file named `moonlit.yml` in the current directory.

## Working Directory

You can specify the working directory with the `-d` or `--working-directory` option:

```bash
moonlit -f ./path/to/moonlit.yml -d ./path/to/working/directory
```

The working directory is used as the base directory for relative paths in your configuration.

## Next Steps

- Explore the [CLI Reference](../../reference/cli.md) for command-line options
- Learn about [creating custom plugins](../advanced/custom-plugins.md)
- See the [reference documentation](../../reference/config-file.md) for all configuration options
