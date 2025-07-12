---
title: Installing Moonlit
description: Learn how to install and set up Moonlit for your .NET projects
---

# Installing Moonlit

Moonlit is distributed as a .NET tool via NuGet. This guide will walk you through the installation process and initial setup.

## Prerequisites

Before installing Moonlit, ensure you have the following prerequisites:

- **.NET SDK**: Version 6.0 or later
- **NuGet**: Latest version recommended
- **Git**: Required for most version control operations (used by Git plugin)

## Installing as a Global Tool

The simplest way to install Moonlit is as a global .NET tool, which makes it available from anywhere on your system:

```bash
dotnet tool install --global moonlit-cli
```

After installation, you can verify that Moonlit is installed correctly by running:

```bash
moonlit --version
```

This should display the current version of Moonlit.

## Installing as a Local Tool

If you prefer to install Moonlit as a project-specific tool, you can add it to your project's local tool manifest:

1. If you haven't created a tool manifest yet, create one:

```bash
dotnet new tool-manifest
```

2. Install Moonlit as a local tool:

```bash
dotnet tool install --local moonlit-cli
```

3. You can then run Moonlit using:

```bash
dotnet tool run moonlit
```

## Installing Specific Versions

If you need to install a specific version of Moonlit:

```bash
dotnet tool install --global moonlit-cli --version 1.0.0
```

Replace `1.0.0` with the version you want to install.

## Updating Moonlit

To update Moonlit to the latest version:

```bash
dotnet tool update --global moonlit-cli
```

Or for a local tool:

```bash
dotnet tool update --local moonlit-cli
```

## Uninstalling Moonlit

If you need to uninstall Moonlit:

```bash
dotnet tool uninstall --global moonlit-cli
```

Or for a local tool:

```bash
dotnet tool uninstall --local moonlit-cli
```

## Next Steps

Now that you have Moonlit installed, you can:

- [Create your first pipeline](./quick-start.md) with a step-by-step guide
- Learn about [Moonlit's core concepts](./concepts/how-it-works.md)
- Explore the [CLI reference](../reference/cli.md) for all available commands and options