---
title: Semantic Release Plugin
description: Documentation for the Semantic Release plugin in Moonlit
---

# Semantic Release Plugin

The Semantic Release plugin provides tools for semantic versioning and changelog generation. It allows you to automatically calculate the next version based on commit messages and generate changelogs for your releases.

## Installation

To use the Semantic Release plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
```

## Middlewares

The Semantic Release plugin provides the following middlewares:

### calculate-version

The `calculate-version` middleware calculates the next version based on commit messages and the current branch. It follows the semantic versioning convention (major.minor.patch).

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| branch | string | Yes | - | The current branch name |
| baseVersion | string | Yes | - | The base version to calculate from (e.g., "v1.0.0") |
| commits | array | Yes | - | An array of commits to analyze |
| prereleaseMappings | object | No | - | A mapping of branch names to prerelease identifiers |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| nextVersion | string | The calculated next version |
| isPrerelease | boolean | Whether the calculated version is a prerelease |
| prereleaseName | string | The prerelease identifier (if applicable) |

#### Example

```yaml
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
          main: latest
          develop: staging
          feature/*: dev
    - name: nextStep
      run: some.other-middleware
      config:
        version: $(output:version:nextVersion)
        isPrerelease: $(output:version:isPrerelease)
```

### generate-changelog

The `generate-changelog` middleware generates a changelog from commit messages. It can optionally use OpenAI to generate more readable and comprehensive changelog entries.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| commits | array | Yes | - | An array of commits to include in the changelog |
| openAiKey | string | No | - | An OpenAI API key to use for generating more readable changelog entries |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| entries | string | The generated changelog entries |

#### Example

```yaml
stages:
  analyze:
    - name: items
      run: gh.items-since-commit
      config:
        tag: $(output:tag:commitSha)
    - name: changelog
      run: sr.generate-changelog
      config:
        commits: $(output:items:commits)
        openAiKey: $(OPENAI_API_KEY)
    - name: nextStep
      run: some.other-middleware
      config:
        changelog: $(output:changelog:entries)
```

## Usage in Pipelines

The Semantic Release plugin is commonly used in release pipelines to:

1. Calculate the next version based on commit messages (using `calculate-version`)
2. Generate a changelog for the release (using `generate-changelog`)

These middlewares are typically used in conjunction with the Git and GitHub plugins to create a complete release pipeline.

For a complete example of using the Semantic Release plugin in a pipeline, see the [NuGet Release Pipeline](./examples/nuget-release.md) example.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [GitHub Plugin](./github.md) for GitHub API integration
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins