---
title: Command Line Interface
description: Reference documentation for Moonlit's command line interface
---

# Command Line Interface

Moonlit provides a command-line interface (CLI) that allows you to run your release pipelines. This page documents the available commands and options.

## Basic Usage

```bash
moonlit [options]
```

## Global Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--file` | `-f` | Path to the configuration file | `moonlit.yml` in the current directory |
| `--stages` | `-s` | Comma-separated list of stages to run | All stages |
| `--working-directory` | `-d` | Working directory for the pipeline | Current directory |
| `--verbose` | `-v` | Enable verbose logging | `false` |
| `--help` | `-h` | Show help information | - |
| `--version` | - | Show version information | - |

## Examples

### Run all stages using the default configuration file

```bash
moonlit
```

### Run specific stages

```bash
moonlit -s build,test
```

### Specify a configuration file

```bash
moonlit -f ./path/to/moonlit.yml
```

### Specify a working directory

```bash
moonlit -d ./path/to/working/directory
```

### Combine multiple options

```bash
moonlit -f ./path/to/moonlit.yml -s build,test -d ./path/to/working/directory -v
```

## Environment Variables

Moonlit uses environment variables for sensitive information and configuration. You can set these variables before running Moonlit:

```bash
# For Windows
set GITHUB_TOKEN=your_github_token
set NUGET_API_KEY=your_nuget_api_key

# For macOS/Linux
export GITHUB_TOKEN=your_github_token
export NUGET_API_KEY=your_nuget_api_key
```

These environment variables can then be referenced in your configuration file:

```yaml
config:
  token: $(GITHUB_TOKEN)
  apiKey: $(NUGET_API_KEY)
```

## Exit Codes

Moonlit returns the following exit codes:

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Plugin error |
| 4 | Pipeline execution error |

## Logging

By default, Moonlit logs information to the console. You can enable verbose logging with the `-v` or `--verbose` option to see more detailed information.

The log format includes:
- Timestamp
- Log level (INFO, WARNING, ERROR)
- Message

Example:
```
🚀 Executing release pipeline: My Pipeline
📁 Working Directory: D:\path\to\your\project
⚙ Configuration File: moonlit.yml

[00:17:08]   ===================================================
[00:17:08]   Executing step (Middleware)
[00:17:08]   Configuration: {"key":"value"}
[00:17:08]   ===================================================
[00:17:08]       INFO Log message
[00:17:08]       WARN Warning message
[00:17:08]       ERROR Error message
[00:17:08]   --------------------------------------------------
[00:17:08]   SUCCESS - Execution time: 125 ms.
[00:17:08]   --------------------------------------------------
```

## Advanced Usage

### Running in CI/CD Environments

When running Moonlit in a CI/CD environment, it's recommended to:

1. Set sensitive information as environment variables
2. Use the `-f` option to specify the configuration file
3. Use the `-s` option to run specific stages if needed

Example for GitHub Actions:

```yaml
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions/setup-dotnet@v1
        with:
          dotnet-version: '6.0.x'
      - run: dotnet tool install --global moonlit-cli
      - run: moonlit -f ./moonlit.yml -s build,publish
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          NUGET_API_KEY: ${{ secrets.NUGET_API_KEY }}
```

### Using Moonlit as a Local Tool

If you've installed Moonlit as a local tool, you need to run it using `dotnet tool run`:

```bash
dotnet tool run moonlit -f ./moonlit.yml
```

## Next Steps

- Learn about the [configuration file structure](./config-file.md)
- Explore the [available plugins](../plugins/)
- See [examples](../plugins/examples/nuget-release.md) of complete pipelines
