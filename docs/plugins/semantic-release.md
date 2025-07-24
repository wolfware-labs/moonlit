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
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0-next.5"
    config:
      openAi:
        apiKey: $(OPENAI_API_KEY)
```

## Middlewares

The Semantic Release plugin provides the following middlewares:

### analyze

The `analyze` middleware analyzes commit messages to identify conventional commits and categorize them by type (feat, fix, etc.).

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| commits | array | Yes | - | An array of commits to analyze |
| includeScopes | array | No | - | An array of scopes to include in the analysis |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| conventionalCommits | object | Information about the analyzed commits |
| commitCount | integer | The number of conventional commits found |
| types | object | Counts of commits by type (feat, fix, etc.) |

#### Example

```yaml
stages:
  analyze:
    - name: commits
      run: git.commits
    - name: conventionalCommits
      run: sr.analyze
      haltIf: output.conventionalCommits.commitCount == 0
      config:
        commits: $(output:commits:details)
        includeScopes:
          - myproject
```

### calculate-version

The `calculate-version` middleware calculates the next version based on commit messages and the current branch. It follows the semantic versioning convention (major.minor.patch).

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| branch | string | Yes | - | The current branch name |
| baseVersion | string | Yes | - | The base version to calculate from (e.g., "v1.0.0") |
| prereleaseMappings | object | No | - | A mapping of branch names to prerelease identifiers |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| nextVersion | string | The calculated next version (without prerelease identifier) |
| nextFullVersion | string | The calculated next version (with prerelease identifier if applicable) |
| isPrerelease | boolean | Whether the calculated version is a prerelease |
| prereleaseName | string | The prerelease identifier (if applicable) |
| hasNewVersion | boolean | Whether a new version was calculated |

#### Example

```yaml
stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: tag
      run: git.latest-tag
      config:
        prefix: "v"
    - name: commits
      run: git.commits
    - name: conventionalCommits
      run: sr.analyze
      config:
        commits: $(output:commits:details)
    - name: version
      run: sr.calculate-version
      haltIf: "!output.version.hasNewVersion"
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
        prereleaseMappings:
          main: next
          develop: beta
          feature/*: alpha
    - name: nextStep
      run: some.other-middleware
      config:
        version: $(output:version:nextVersion)
        fullVersion: $(output:version:nextFullVersion)
        isPrerelease: $(output:version:isPrerelease)
```

### generate-changelog

The `generate-changelog` middleware generates a changelog from conventional commits. It can use OpenAI to generate more readable and comprehensive changelog entries.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| commits | array | No | - | An array of commits to include in the changelog (if not provided, uses the commits from the analyze middleware) |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| categories | object | The generated changelog entries organized by category (features, fixes, etc.) |
| markdown | string | The complete changelog in markdown format |

#### Example

```yaml
stages:
  analyze:
    - name: commits
      run: git.commits
    - name: conventionalCommits
      run: sr.analyze
      config:
        commits: $(output:commits:details)
    - name: changelog
      run: sr.generate-changelog
    - name: createRelease
      run: gh.create-release
      config:
        changelog: $(output:changelog:categories)
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