# Moonlit Plugin

The Moonlit plugin is designed for managing Moonlit release files as submodules in the Moonlit release automation system.

::: warning
This plugin is currently under development and not fully implemented yet. The core functionality is planned but not available in the current version. The implementation is incomplete with the main middleware method throwing a `NotImplementedException`.
:::

## Installation

Add the Moonlit plugin to your Moonlit configuration file:

```yaml
plugins:
  - name: "moonlit"
    url: "nuget://Wolfware.Moonlit.Plugins.Moonlit/1.0.0"
```

## Planned Features

The Moonlit plugin is intended to provide the following features:

- Managing Moonlit release configuration files as submodules
- Providing utilities for working with Moonlit-specific release processes
- Simplifying the management of complex Moonlit release pipelines

## Current Status

This plugin is in early development. The implementation is incomplete and using it in production environments is not recommended at this time.

## Future Development

Future versions of this plugin will include:

- Complete implementation of the middleware pipeline
- Documentation of available commands and configuration options
- Examples of common use cases
