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
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
  - name: "dotnet"
    url: "nuget://Wolfware.Moonlit.Plugins.DotNet/1.0.0"

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
🚀 Executing release pipeline: My First Pipeline
📁 Working Directory: D:\path\to\your\project
⚙ Configuration File: moonlit.yml

[00:17:08]   ===================================================
[00:17:08]   Executing repo (GetRepositoryContext)
[00:17:08]   Configuration: {}
[00:17:08]   ===================================================
[00:17:08]       INFO Current branch: main
[00:17:08]       INFO Remote URL: https://github.com/username/repo.git
[00:17:08]   --------------------------------------------------
[00:17:08]   SUCCESS - Execution time: 125 ms.
[00:17:08]   --------------------------------------------------
[00:17:08]              
[00:17:08]              
[00:17:08]   ===================================================
[00:17:08]   Executing build (DotNetBuildMiddleware)
[00:17:08]   Configuration: {"project":"./src/MyProject.csproj","configuration":"Release"}
[00:17:08]   ===================================================
[00:17:09]       INFO Building project: ./src/MyProject.csproj
[00:17:10]       INFO Build completed successfully
[00:17:10]   --------------------------------------------------
[00:17:10]   SUCCESS - Execution time: 2105 ms.
[00:17:10]   --------------------------------------------------
[00:17:10]              
[00:17:10]              
[00:17:10]   ===================================================
[00:17:10]   Executing tag (GetLatestTag)
[00:17:10]   Configuration: {"prefix":"v"}
[00:17:10]   ===================================================
[00:17:10]       INFO Found tag: v1.0.0
[00:17:10]   --------------------------------------------------
[00:17:10]   SUCCESS - Execution time: 356 ms.
[00:17:10]   --------------------------------------------------
[00:17:10]              
[00:17:10]              
[00:17:10]   ===================================================
[00:17:10]   Executing version (CalculateVersion)
[00:17:10]   Configuration: {"branch":"main","baseVersion":"v1.0.0"}
[00:17:10]   ===================================================
[00:17:10]       INFO Calculating next version
[00:17:10]       INFO Next version calculated: 1.1.0
[00:17:10]   --------------------------------------------------
[00:17:10]   SUCCESS - Execution time: 78 ms.
[00:17:10]   --------------------------------------------------
[00:17:10]              
[00:17:10]              
[00:17:10]   ===================================================
[00:17:10]   Executing createRelease (CreateRelease)
[00:17:10]   Configuration: {"name":"Release 1.1.0","tag":"v1.1.0"}
[00:17:10]   ===================================================
[00:17:11]       INFO Creating GitHub release: v1.1.0
[00:17:11]       INFO Release created successfully
[00:17:11]   --------------------------------------------------
[00:17:11]   SUCCESS - Execution time: 1254 ms.
[00:17:11]   --------------------------------------------------
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
