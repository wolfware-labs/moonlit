---
title: Stages and Steps
description: Learn how Moonlit organizes pipeline execution with stages and steps
---

# Stages and Steps

Moonlit organizes your release pipeline into stages and steps, providing a structured way to define and execute your automation tasks. This page explains how stages and steps work in Moonlit.

## Stages

Stages are logical groupings of steps in your release pipeline. They help you organize your pipeline into distinct phases, such as build, test, publish, and deploy.

### Defining Stages

Stages are defined in your Moonlit configuration file under the `stages` section:

```yaml
stages:
  build:
    # Steps for the build stage
  
  test:
    # Steps for the test stage
  
  publish:
    # Steps for the publish stage
```

Each stage has a unique name and contains a list of steps to execute.

### Stage Execution

By default, Moonlit executes all stages in the order they are defined in your configuration file. However, you can specify which stages to run using the `-s` or `--stages` command-line option:

```bash
moonlit -f moonlit.yml -s build,test
```

This command will only execute the `build` and `test` stages, skipping any other stages defined in the configuration.

### Stage Dependencies

Stages are executed in the order they are defined in your configuration file. There is an implicit dependency between stages, where each stage depends on the successful completion of the previous stage.

If a step in a stage fails, Moonlit will stop the execution of the pipeline by default, unless you configure the step to continue on error.

## Steps

Steps are individual tasks within a stage. Each step executes a specific middleware provided by a plugin.

### Defining Steps

Steps are defined within a stage:

```yaml
stages:
  build:
    - name: repo
      run: git.repo-context
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
```

Each step has:

- **name**: A unique identifier for the step
- **run**: The middleware to execute, in the format `pluginName.middlewareName`
- **config** (optional): Configuration settings for the middleware

### Step Execution

Steps within a stage are executed sequentially in the order they are defined. Each step:

1. Receives the pipeline context
2. Executes its middleware
3. Updates the context with its results
4. Passes the context to the next step

### Step Output

Steps can produce output that can be used by later steps. This output is stored in the pipeline context and can be accessed using the `$(output:stepName:propertyName)` syntax:

```yaml
stages:
  analyze:
    - name: repo
      run: git.repo-context
    
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
```

In this example, the `version` step uses the `branch` output from the `repo` step.

### Step Configuration

Steps can be configured using the `config` property:

```yaml
- name: build
  run: dotnet.build
  config:
    project: "./src/MyProject.csproj"
    configuration: "Release"
    verbosity: "minimal"
```

The available configuration options depend on the middleware being executed. Each middleware defines its own configuration schema.

### Conditional Steps

You can make steps conditional using the `condition` property:

```yaml
- name: deployToProduction
  run: deploy.azure
  condition: $(output:repo:branch) == 'main'
  config:
    environment: "production"
```

In this example, the `deployToProduction` step will only execute if the current branch is `main`.

### Error Handling

By default, if a step fails, Moonlit will stop the execution of the pipeline. However, you can configure a step to continue on error using the `continueOnError` property:

```yaml
- name: notify
  run: slack.send-notification
  continueOnError: true
  config:
    channel: "#releases"
    message: "Build completed"
```

In this example, if the `notify` step fails, Moonlit will log the error but continue with the next step or stage.

## Example: Complete Pipeline

Here's an example of a complete pipeline with multiple stages and steps:

```yaml
name: "NuGet Package Release"

plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
  - name: "nuget"
    url: "nuget://Wolfware.Moonlit.Plugins.Nuget/1.0.0"

stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: gh.latest-tag
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)

  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        configuration: "Release"

  publish:
    - name: pack
      run: nuget.pack
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextVersion)
    
    - name: push
      run: nuget.push
      config:
        package: $(output:pack:packagePath)
        source: "https://api.nuget.org/v3/index.json"
        apiKey: $(NUGET_API_KEY)
```

## Next Steps

- Learn about [Configuration](./configuration.md) options in more detail
- Explore the [CLI Reference](../../reference/cli.md) for command-line options
- See how to [create your own plugins](../advanced/custom-plugins.md) with custom middlewares