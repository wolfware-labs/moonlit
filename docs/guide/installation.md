---
title: Installing Moonlit
description: Learn how to install and set up Moonlit for your projects
---

# Installing Moonlit

Moonlit is distributed as a .NET tool via NuGet. This guide will walk you through the installation process and initial setup.

## Prerequisites

Before installing Moonlit, ensure you have the following prerequisites:

- **.NET SDK**: Version 9.0 or later
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


## Uninstalling Moonlit

If you need to uninstall Moonlit:

```bash
dotnet tool uninstall --global moonlit-cli
```


## Next Steps

Now that you have Moonlit installed, you can:

- [Create your first pipeline](./quick-start.md) with a step-by-step guide
- Learn about [Moonlit's core concepts](./concepts/how-it-works.md)
- Explore the [CLI reference](../reference/cli.md) for all available commands and options
