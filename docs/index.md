---
layout: home

hero:
  name: "Moonlit"
  text: "Modern .NET Release Pipeline"
  tagline: A powerful, extensible build and release automation tool for .NET projects
  image:
    src: /logo.png
    alt: Moonlit
  actions:
    - theme: brand
      text: Get Started
      link: /guide/
    - theme: alt
      text: View on GitHub
      link: https://github.com/wolfware/moonlit
    - theme: alt
      text: NuGet Package
      link: https://www.nuget.org/packages/moonlit-cli

features:
  - icon: 🚀
    title: Streamlined Releases
    details: Automate your entire release process from code to deployment with a single YAML configuration file.

  - icon: 🧩
    title: Plugin Ecosystem
    details: Extend functionality with plugins for Git, GitHub, Semantic Versioning, Slack, NuGet, Docker, and NPM.

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

Moonlit is a .NET-based build and release automation tool designed to simplify and streamline your release pipeline. It provides a flexible, plugin-based architecture that allows you to automate complex release processes with a simple YAML configuration file.

## Quick Example

```yaml
name: "NuGet Package Release"

plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"

stages:
  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"

  publish:
    - name: pack
      run: nuget.pack
      config:
        project: "./src/MyProject.csproj"
```

## Installation

```bash
dotnet tool install --global moonlit-cli
```

[Learn more about Moonlit](/guide/)
