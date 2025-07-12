---
title: Quick Start Guide
description: Create your first Moonlit pipeline in minutes
---

# Quick Start Guide

This guide will walk you through creating a simple Moonlit pipeline to help you get started quickly. We'll create a basic pipeline that builds a .NET project and creates a GitHub release.

## Prerequisites

Before you begin, make sure you have:

- [Installed Moonlit](./installation.md)
- A .NET project with a Git repository
- A GitHub account and personal access token (for GitHub operations)

## Step 1: Create a Configuration File

Create a file named `moonlit.yml` in the root of your project with the following content:

```yaml
name: "My First Pipeline"

plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)

stages:
  build:
    - name: repo
      run: git.repo-context
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        configuration: "Release"

  publish:
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
    - name: createRelease
      run: gh.create-release
      config:
        name: "Release $(output:version:nextVersion)"
        tag: "v$(output:version:nextVersion)"
```

## Step 2: Set Environment Variables

Moonlit uses environment variables for sensitive information. Set the GitHub token:

```bash
# For Windows
set GITHUB_TOKEN=your_github_token

# For macOS/Linux
export GITHUB_TOKEN=your_github_token
```

## Step 3: Run the Pipeline

Now you can run your pipeline:

```bash
moonlit -f moonlit.yml
```

This will execute all stages in the pipeline. If you want to run only specific stages:

```bash
moonlit -f moonlit.yml -s build
```

## Step 4: Examine the Output

Moonlit will display the progress of each step in the pipeline. If everything is configured correctly, you should see output similar to:

```
[INFO] Starting pipeline: My First Pipeline
[INFO] Loading plugins...
[INFO] Running stage: build
[INFO] Step: repo
[INFO] Step: build
[INFO] Running stage: publish
[INFO] Step: tag
[INFO] Step: version
[INFO] Step: createRelease
[INFO] Pipeline completed successfully
```

## Understanding the Configuration

Let's break down the configuration file:

- **name**: The name of your pipeline
- **plugins**: The list of plugins to use in your pipeline
  - Each plugin has a name and a URL (NuGet package)
  - Some plugins require configuration (like the GitHub token)
- **stages**: Logical groupings of steps
  - Each stage contains a list of steps
  - Steps are executed in the order they are defined

## Using Output from Previous Steps

One powerful feature of Moonlit is the ability to use output from previous steps:

```yaml
$(output:stepName:propertyName)
```

In our example, we used:
- `$(output:repo:branch)` to get the current branch from the repo step
- `$(output:tag:name)` to get the latest tag name
- `$(output:version:nextVersion)` to get the calculated version

## Next Steps

Now that you've created your first pipeline, you can:

- Learn about [Moonlit's core concepts](./concepts/how-it-works.md)
- Explore the [configuration file structure](../reference/config-file.md)
- Check out the [available plugins](../plugins/) and their capabilities
- Learn how to [create custom plugins](./advanced/custom-plugins.md)