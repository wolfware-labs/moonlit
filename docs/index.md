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

## Quick Example

This is just one example of what Moonlit can do. Moonlit can work with various project types and technologies, not just .NET projects. See the [Plugins](/plugins/) section for more examples, including Docker and NPM integrations.

```yaml
name: "NuGet Package Release"

variables:
  projectPath: "./src/MyProject.csproj"
  nugetSource: "https://api.nuget.org/v3/index.json"

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
  - name: "slack"
    url: "nuget://Wolfware.Moonlit.Plugins.Slack/1.0.0"
    config:
      token: $(SLACK_TOKEN)

stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"
    - name: items
      run: gh.items-since-commit
      config:
        tag: $(output:tag:commitSha)
    - name: version
      run: sr.calculate-version
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
        commits: $(output:items:commits)
        prereleaseMappings:
          main: next
          develop: beta
          feature/*: alpha

  build:
    - name: build
      run: dotnet.build
      config:
        project: $(vars:projectPath)
        configuration: "Release"

  test:
    - name: test
      run: dotnet.test
      config:
        project: $(vars:projectPath)
        configuration: "Debug"

  publish:
    - name: pack
      run: dotnet.pack
      config:
        project: $(vars:projectPath)
        version: $(output:version:nextVersion)
    - name: push
      run: dotnet.push
      condition: $(output:repo:branch) == 'main' || $(output:repo:branch) == 'develop'
      config:
        package: $(output:pack:packagePath)
        source: $(vars:nugetSource)
    - name: changelog
      run: sr.generate-changelog
      config:
        commits: $(output:items:commits)
    - name: release
      run: gh.create-release
      condition: $(output:repo:branch) == 'main'
      config:
        name: "Release $(output:version:nextVersion)"
        tag: "v$(output:version:nextVersion)"
        changelog: $(output:changelog:entries)
        prerelease: $(output:version:isPrerelease)

  notify:
    - name: notify-slack
      run: slack.send-notification
      config:
        channel: "#releases"
        message: ":rocket: New Release - $(output:version:nextVersion) is now available! :tada:"
```

[Learn more about Moonlit](/guide/)
