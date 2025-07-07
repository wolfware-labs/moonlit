# Moonlit Git Plugin

[![NuGet](https://img.shields.io/nuget/v/Wolfware.Moonlit.Plugins.Git.svg)](https://www.nuget.org/packages/Wolfware.Moonlit.Plugins.Git/)

## Overview

The Moonlit Git Plugin provides Git repository integration and version control capabilities for the Moonlit framework. This plugin allows your Moonlit applications to interact with Git repositories, retrieve repository information, and incorporate Git-based workflows into your release pipelines.

## Features

- Retrieve Git repository context including branch name and remote URL
- Integrate Git repository information into your Moonlit pipeline
- Simple API for accessing Git repository details
- Built on top of LibGit2Sharp for reliable Git operations

## Installation

Install the Moonlit Git Plugin using the NuGet Package Manager:

```
Install-Package Wolfware.Moonlit.Plugins.Git
```

Or via the .NET CLI:

```
dotnet add package Wolfware.Moonlit.Plugins.Git
```

## Usage

### Basic Setup

Add the Git plugin to your Moonlit release pipeline configuration:

```yaml
plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
```

## Requirements

- .NET 9.0 or higher
- Moonlit Plugins framework
- LibGit2Sharp (automatically installed as a dependency)

## License

This project is licensed under the terms specified in the LICENSE.txt file included with this package.

## About Wolfware

Moonlit is a product of Wolfware LLC, providing modern tools for streamlined development workflows.

© 2025 Wolfware LLC. All rights reserved.