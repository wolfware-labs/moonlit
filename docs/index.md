---
layout: home

hero:
  name: "Moonlit"
  text: "Bring light to your release process"
  tagline: A powerful, extensible build and release automation tool for modern development workflows.
  image:
    src: /logo.png
    alt: Moonlit
  actions:
    - theme: brand
      text: Get Started
      link: /guide/
    - theme: alt
      text: NuGet Package
      link: https://www.nuget.org/packages/moonlit-cli

features:
  - icon: 🚀
    title: Streamlined Releases
    details: Automate your entire release process from code to deployment with a single YAML configuration file.

  - icon: 🧩
    title: Plugin Ecosystem
    details: Extend functionality with plugins for Git, GitHub, Semantic Versioning, Slack, NuGet, Docker, NPM, and many more.

  - icon: 🔄
    title: Pipeline Architecture
    details: Define stages and steps in your release process with a flexible middleware pipeline system.

  - icon: 🛠️
    title: Highly Configurable
    details: Customize every aspect of your release process with a powerful configuration system.

  - icon: 📦
    title: NuGet Integration
    details: Distribute and consume plugins as NuGet packages for seamless integration.

  - icon: 🔌
    title: Easy to Extend
    details: Create your own plugins to integrate with any tool or service in your development workflow.
---

## What is Moonlit?

Moonlit is a build and release automation tool designed to simplify and streamline your release pipeline. It provides a flexible, plugin-based architecture that allows you to automate complex release processes for various project types with a simple YAML configuration file. Moonlit can work with many different technologies including Docker, NPM, and more.

Even this docs site you are reading right now is built with Moonlit! 😮 All the nuget packages part of the Moonlit toolset, including the CLI are built with Moonlit as well! 🤯

## Installation

```bash
dotnet tool install --global moonlit-cli
```

To install a prerelease version:

```bash
dotnet tool install --global moonlit-cli --prerelease
```

## Quick Example

This is just one example of what Moonlit can do. Moonlit can work with various project types and technologies, not just .NET projects. See the [Plugins](/plugins/) section for more examples, including Docker and NPM integrations.

```yaml
name: "NuGet Package Release"  # Name of the pipeline configuration

# Define reusable variables that can be referenced throughout the pipeline
variables:
  projectPath: "./src/MyProject.csproj"  # Path to the .NET project file
  nugetSource: "https://api.nuget.org/v3/index.json"  # NuGet repository URL

# Register plugins that provide middlewares for the pipeline
plugins:
  # Git plugin for repository operations
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"

  # GitHub plugin for interacting with GitHub API
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)  # Uses environment variable for authentication

  # Semantic Release plugin for versioning and changelog generation
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"

  # .NET plugin for building, packing, and publishing NuGet packages
  - name: "dotnet"
    url: "nuget://Wolfware.Moonlit.Plugins.Dotnet/1.0.0"
    config:
      apiKey: $(NUGET_API_KEY)  # Uses environment variable for NuGet authentication

  # Slack plugin for sending notifications
  - name: "slack"
    url: "nuget://Wolfware.Moonlit.Plugins.Slack/1.0.0"
    config:
      token: $(SLACK_TOKEN)  # Uses environment variable for Slack authentication

# Define the stages of the pipeline, executed in sequence
stages:
  # First stage: Analyze the repository and determine the next version
  analyze:
    # Get information about the Git repository
    - name: repo
      run: git.repo-context

    # Get the latest tag from GitHub
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"  # Only consider tags starting with "v"

    # Get all commits, PRs, and issues since the last tag
    - name: items
      run: gh.items-since-commit
      config:
        tag: $(output:tag:commitSha)  # Uses output from the previous step

    # Calculate the next version based on semantic versioning
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)  # Uses branch name from repo step
        baseVersion: $(output:tag:name)  # Uses tag name from tag step
        commits: $(output:items:commits)  # Uses commits from items step
        prereleaseMappings:  # Define prerelease identifiers based on branch
          main: next
          develop: beta
          feature/*: alpha  # Uses wildcard pattern for feature branches

  # Second stage: Build the .NET project
  build:
    - name: build
      run: dotnet.build
      config:
        project: $(vars:projectPath)  # References variable defined at the top
        configuration: "Release"  # Build in Release mode

  # Third stage: Run tests on the .NET project
  test:
    - name: test
      run: dotnet.test
      config:
        project: $(vars:projectPath)  # References variable defined at the top
        configuration: "Debug"  # Test in Debug mode for better diagnostics

  # Fourth stage: Package and publish the NuGet package
  publish:
    # Create a NuGet package with the calculated version
    - name: pack
      run: dotnet.pack
      config:
        project: $(vars:projectPath)  # References variable defined at the top
        version: $(output:version:nextVersion)  # Uses version from analyze stage

    # Push the package to NuGet repository, but only for main or develop branches
    - name: push
      run: dotnet.push
      condition: $(output:repo:branch) == 'main' || $(output:repo:branch) == 'develop'  # Only run on main or develop
      haltIf: $(output:version:isPrerelease) == true && $(output:repo:branch) != 'develop'  # Stop if prerelease on non-develop branch
      config:
        package: $(output:pack:packagePath)  # Uses package path from pack step
        source: $(vars:nugetSource)  # References variable defined at the top

    # Generate a changelog from the commits
    - name: changelog
      run: sr.generate-changelog
      config:
        commits: $(output:items:commits)  # Uses commits from analyze stage
        openAiKey: $(OPENAI_API_KEY)  # Uses OpenAI API key for AI-generated changelogs

    # Create a GitHub release, but only for the main branch
    - name: release
      run: gh.create-release
      condition: $(output:repo:branch) == 'main'  # Only run on main branch
      config:
        name: "Release $(output:version:nextVersion)"  # Uses version in release name
        tag: "v$(output:version:nextVersion)"  # Creates a tag with v-prefix
        changelog: $(output:changelog:entries)  # Uses generated changelog
        prerelease: $(output:version:isPrerelease)  # Marks as prerelease if version is prerelease

  # Final stage: Send a notification about the release
  notify:
    - name: notify-slack
      run: slack.send-notification
      config:
        channel: "#releases"  # Slack channel to notify
        message: ":rocket: New Release - $(output:version:nextVersion) is now available! :tada:"  # Message with version
```

[Learn more about Moonlit](/guide/)
