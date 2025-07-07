# Moonlit Semantic Release Plugin

[![NuGet](https://img.shields.io/nuget/v/Wolfware.Moonlit.Plugins.SemanticRelease.svg)](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.SemanticRelease/)

## Overview

The Moonlit Semantic Release Plugin adds automatic versioning and changelog generation capabilities to the Moonlit
release automation framework. This plugin analyzes your commit history using conventional commit standards to determine
the next semantic version and generate comprehensive changelogs.

## Features

- Automatic semantic version calculation based on commit history
- Smart version bumping following SemVer 2.0 principles
- AI-powered changelog generation from commit messages
- Customizable version calculation rules
- Branch-specific prerelease version handling

## Installation

Install the Moonlit Semantic Release Plugin using the NuGet Package Manager:

```
Install-Package Wolfware.Moonlit.Plugins.SemanticRelease
```

Or via the .NET CLI:
~~~~
```
dotnet add package Wolfware.Moonlit.Plugins.SemanticRelease
```

## Usage

### Basic Setup

Add the Semantic Release plugin to your Moonlit release pipeline configuration:

```yaml
plugins:
  - name: "semantic-release"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
    configuration:
      openAiKey: "YOUR_OPENAI_API_KEY"
```

### Available Middlewares

The plugin provides two primary middlewares:

- **calculate-version**: Determines the next semantic version based on commit history
- **generate-changelog**: Creates a structured changelog using AI analysis of commits

### Version Calculation Configuration

Customize version calculation with these options:

```yaml
middlewares:
  - name: "calculate-version"
    configuration:
      initialVersion: "1.0.0"
      baseVersion: "1.2.3"  # Optional, uses initialVersion if not specified
      prereleaseMappings:
        develop: "alpha"
        staging: "beta"
        main: ""
```

### Changelog Generation

Generate comprehensive changelogs with AI assistance:

```yaml
middlewares:
  - name: "generate-changelog"
    configuration:
      openAiKey: "YOUR_OPENAI_API_KEY"
```

## Requirements

- .NET 9.0 or higher
- Moonlit Plugins framework
- LibGit2Sharp (automatically installed as a dependency)
- Semver (automatically installed as a dependency)
- OpenAI API key for changelog generation

## License

This project is licensed under the terms specified in the LICENSE.txt file included with this package.

## About Wolfware

Moonlit is a product of Wolfware LLC, providing modern tools for streamlined development workflows.

© 2025 Wolfware LLC. All rights reserved.