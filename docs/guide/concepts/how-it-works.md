---
title: How Moonlit Works
description: Understand the architecture and core concepts of Moonlit
---

# How Moonlit Works

This page explains the architecture and core concepts of Moonlit, helping you understand how the tool processes your configuration and executes your pipeline.

## Architecture Overview

Moonlit follows a plugin-based architecture with a middleware pipeline pattern. Here's a high-level overview of how Moonlit works:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│                 │     │                 │     │                 │
│  Configuration  │────▶│  Plugin Loader  │────▶│  Pipeline       │
│  Parser         │     │                 │     │  Executor       │
│                 │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                        │
                                                        ▼
                                         ┌─────────────────────────────┐
                                         │                             │
                                         │  Stage 1                    │
                                         │  ┌───────┐ ┌───────┐        │
                                         │  │Step 1 │▶│Step 2 │▶...    │
                                         │  └───────┘ └───────┘        │
                                         │                             │
                                         └─────────────────────────────┘
                                                        │
                                                        ▼
                                         ┌─────────────────────────────┐
                                         │                             │
                                         │  Stage 2                    │
                                         │  ┌───────┐ ┌───────┐        │
                                         │  │Step 1 │▶│Step 2 │▶...    │
                                         │  └───────┘ └───────┘        │
                                         │                             │
                                         └─────────────────────────────┘
```

## Core Components

### Configuration Parser

The Configuration Parser reads your YAML file and converts it into a structured object model that Moonlit can process. It handles:

- Validating the YAML structure
- Resolving environment variables
- Parsing plugin configurations
- Organizing stages and steps

### Plugin Loader

The Plugin Loader is responsible for:

- Downloading and loading plugins from NuGet
- Initializing plugin startup classes
- Registering middleware components
- Setting up dependency injection

Each plugin must implement the `IPluginStartup` interface or inherit from the `PluginStartup` class to be properly loaded.

### Pipeline Executor

The Pipeline Executor manages the execution of your pipeline:

- It organizes steps into stages
- It executes stages in the order defined in your configuration
- It passes the shared context between steps
- It handles errors and provides logging

### Middleware Pipeline

Each step in your pipeline is a middleware that:

- Receives the pipeline context
- Performs its specific action
- Updates the context with its results
- Passes the context to the next middleware

This pattern allows for a flexible and extensible pipeline where each step can build upon the results of previous steps.

## Execution Flow

When you run Moonlit, the following sequence occurs:

1. **Configuration Loading**: Moonlit reads and parses your YAML configuration file
2. **Plugin Resolution**: Moonlit resolves and loads the plugins specified in your configuration
3. **Pipeline Initialization**: Moonlit sets up the pipeline with all the stages and steps
4. **Stage Execution**: Moonlit executes each stage in sequence (or only the stages you specified)
5. **Step Execution**: Within each stage, Moonlit executes each step in sequence
6. **Result Reporting**: Moonlit reports the results of the pipeline execution

## Context Sharing

One of the key features of Moonlit is the ability to share data between steps. This is done through:

1. **Output Variables**: Each step can produce output variables that can be referenced by later steps
2. **Configuration System**: Moonlit's configuration system allows for dynamic values using the `$(output:step:property)` syntax
3. **Dependency Injection**: Moonlit uses .NET's dependency injection system to share services between steps

## Error Handling

Moonlit includes robust error handling:

- If a step fails, the pipeline execution stops by default
- Errors are logged with detailed information
- You can configure steps to continue on error if needed

## Next Steps

- Learn about the [Plugins System](./plugins.md)
- Understand [Stages and Steps](./stages-steps.md) in more detail
- Explore the [Configuration](./configuration.md) options