---
title: Quick Start Guide
description: Create your first Moonlit pipeline in minutes
---

# Quick Start Guide

This guide will walk you through creating a simple Moonlit pipeline to help you get started quickly. We'll create a basic pipeline that builds a project and creates a GitHub release. While this example uses a .NET project, Moonlit works with various project types and technologies.

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
        prefix: "v"
    - name: commits
      run: git.commits
    - name: conventionalCommits
      run: sr.analyze
      config:
        commits: $(output:commits:details)
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
    - name: changelog
      run: sr.generate-changelog

  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextFullVersion)
        configuration: "Release"

  release:
    - name: createRelease
      run: gh.create-release
      config:
        name: "Release $(output:version:nextVersion)"
        tag: "v$(output:version:nextVersion)"
        changelog: $(output:changelog:categories)
        prerelease: $(output:version:isPrerelease)
```

## Step 2: Set Environment Variables

Moonlit uses environment variables for sensitive information. Set the required tokens and API keys:

```bash
# For Windows
set GITHUB_TOKEN=your_github_token
set OPENAI_API_KEY=your_openai_api_key
set NUGET_API_KEY=your_nuget_api_key

# For macOS/Linux
export GITHUB_TOKEN=your_github_token
export OPENAI_API_KEY=your_openai_api_key
export NUGET_API_KEY=your_nuget_api_key
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

[00:17:08]   ================================================================================
[00:17:08]   Step: repo
[00:17:08]   Middleware: Wolfware.Moonlit.Plugins.Git.Middlewares.GetRepositoryContext
[00:17:08]   Version: 1.0.0
[00:17:08]   ================================================================================
[00:17:08]       INFO Current branch: main
[00:17:08]       INFO Remote URL: https://github.com/username/repo.git
[00:17:08]   --------------------------------------------------------------------------------
[00:17:08]   SUCCESS - Execution time: 125 ms.
[00:17:08]   --------------------------------------------------------------------------------
[00:17:08]              
[00:17:08]              
[00:17:08]   ================================================================================
[00:17:08]   Step: build
[00:17:08]   Middleware: Wolfware.Moonlit.Plugins.Dotnet.Middlewares.DotNetBuildMiddleware
[00:17:08]   Version: 1.0.0
[00:17:08]   ================================================================================
[00:17:09]       INFO Building project: ./src/MyProject.csproj
[00:17:10]       INFO Build completed successfully
[00:17:10]   --------------------------------------------------------------------------------
[00:17:10]   SUCCESS - Execution time: 2105 ms.
[00:17:10]   --------------------------------------------------------------------------------
[00:17:10]              
[00:17:10]              
[00:17:10]   ================================================================================
[00:17:10]   Step: tag
[00:17:10]   Middleware: Wolfware.Moonlit.Plugins.Git.Middlewares.GetLatestTag
[00:17:10]   Version: 1.0.0
[00:17:10]   ================================================================================
[00:17:10]       INFO Found tag: v1.0.0
[00:17:10]   --------------------------------------------------------------------------------
[00:17:10]   SUCCESS - Execution time: 356 ms.
[00:17:10]   --------------------------------------------------------------------------------
[00:17:10]              
[00:17:10]              
[00:17:10]   ================================================================================
[00:17:10]   Step: version
[00:17:10]   Middleware: Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares.CalculateVersion
[00:17:10]   Version: 1.0.0
[00:17:10]   ================================================================================
[00:17:10]       INFO Calculating next version
[00:17:10]       INFO Next version calculated: 1.1.0
[00:17:10]   --------------------------------------------------------------------------------
[00:17:10]   SUCCESS - Execution time: 78 ms.
[00:17:10]   --------------------------------------------------------------------------------
[00:17:10]              
[00:17:10]              
[00:17:10]   ================================================================================
[00:17:10]   Step: createRelease
[00:17:10]   Middleware: Wolfware.Moonlit.Plugins.Github.Middlewares.CreateRelease
[00:17:10]   Version: 1.0.0
[00:17:10]   ================================================================================
[00:17:11]       INFO Creating GitHub release: v1.1.0
[00:17:11]       INFO Release created successfully
[00:17:11]   --------------------------------------------------------------------------------
[00:17:11]   SUCCESS - Execution time: 1254 ms.
[00:17:11]   --------------------------------------------------------------------------------
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
