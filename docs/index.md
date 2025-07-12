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

## Quick Example

This is just one example of what Moonlit can do. Moonlit can work with various project types and technologies, not just .NET projects. See the [Plugins](/plugins/) section for more examples, including Docker and NPM integrations.

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
