---
title: GitHub Plugin
description: Documentation for the GitHub plugin in Moonlit
---

# GitHub Plugin

The GitHub plugin provides integration with GitHub. It allows you to retrieve information from GitHub repositories, create releases, and work with issues and pull requests.

## Installation

To use the GitHub plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
```

Note that the GitHub plugin requires a GitHub token to authenticate with the GitHub API. You can set this token as an environment variable and reference it in your configuration file.

## Middlewares

The GitHub plugin provides the following middlewares:

### latest-tag

The `latest-tag` middleware retrieves the latest tag from GitHub. It can be filtered to only include tags with a specific prefix.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| prefix | string | No | - | A prefix to filter tags (e.g., "v" to only include tags like "v1.0.0") |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| name | string | The name of the latest tag (e.g., "v1.0.0") |
| commitSha | string | The commit SHA that the tag points to |

#### Example

```yaml
stages:
  analyze:
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"
    - name: nextStep
      run: some.other-middleware
      config:
        tagName: $(output:tag:name)
        commitSha: $(output:tag:commitSha)
```

### items-since-commit

The `items-since-commit` middleware retrieves commits, pull requests, and issues that have been created or merged since a specific commit.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| tag | string | Yes | - | The commit SHA to use as the starting point |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| commits | array | An array of commits since the specified commit |
| pullRequests | array | An array of pull requests that have been merged since the specified commit |
| issues | array | An array of issues that have been closed since the specified commit |

#### Example

```yaml
stages:
  analyze:
    - name: tag
      run: gh.latest-tag
      config:
        prefix: "v"
    - name: items
      run: gh.items-since-commit
      config:
        tag: $(output:tag:commitSha)
    - name: nextStep
      run: some.other-middleware
      config:
        commits: $(output:items:commits)
        pullRequests: $(output:items:pullRequests)
        issues: $(output:items:issues)
```

### create-release

The `create-release` middleware creates a GitHub release.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| name | string | Yes | - | The name of the release |
| tag | string | Yes | - | The tag to create the release from |
| label | string | No | - | A label for the release |
| changelog | string | No | - | The changelog for the release |
| prerelease | boolean | No | false | Whether the release is a prerelease |
| pullRequests | array | No | - | Pull requests to include in the release |
| issues | array | No | - | Issues to include in the release |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| url | string | The URL of the created release |
| name | string | The name of the created release |

#### Example

```yaml
stages:
  release:
    - name: createRelease
      run: gh.create-release
      config:
        name: "Release $(output:version:nextVersion)"
        tag: $(output:version:nextVersion)
        label: "released on @$(output:repo:branch)"
        changelog: $(output:changelog:entries)
        prerelease: $(output:version:isPrerelease)
        pullRequests: $(output:items:pullRequests)
        issues: $(output:items:issues)
    - name: nextStep
      run: some.other-middleware
      config:
        releaseUrl: $(output:createRelease:url)
        releaseName: $(output:createRelease:name)
```

### write-variables

The `write-variables` middleware allows you to set output and environment variables in your GitHub workflow.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| output | dictionary | No | {} | A dictionary of output variables to set |
| environment | dictionary | No | {} | A dictionary of environment variables to set |

#### Outputs

This middleware does not produce any outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  setup:
    - name: setVariables
      run: gh.write-variables
      config:
        output:
          REPO_NAME: "my-repo"
          VERSION: "1.0.0"
        environment:
          GITHUB_ENV: "production"
          DEPLOY_TARGET: "main"
```

## Usage in Pipelines

The GitHub plugin is commonly used in release pipelines to:

1. Get information about the latest release (using `latest-tag`)
2. Gather information about changes since the last release (using `items-since-commit`)
3. Create a new release with a changelog and links to pull requests and issues (using `create-release`)

For a complete example of using the GitHub plugin in a pipeline, see the [NuGet Release Pipeline](./examples/nuget-release.md) example.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [Semantic Release Plugin](./semantic-release.md) for semantic versioning
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins
