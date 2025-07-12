---
title: Git Plugin
description: Documentation for the Git plugin in Moonlit
---

# Git Plugin

The Git plugin provides integration with Git repositories. It allows you to get information about the current repository and perform Git operations like commit, tag, and push.

## Installation

To use the Git plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
```

## Middlewares

The Git plugin provides the following middlewares:

### repo-context

The `repo-context` middleware retrieves information about the current Git repository, such as the branch name and remote URL. This information can be used in subsequent steps of your pipeline.

#### Inputs

This middleware does not require any inputs.

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| *None* | | | | |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| branch | string | The name of the current Git branch |
| remoteUrl | string | The URL of the remote Git repository |

#### Example

```yaml
stages:
  analyze:
    - name: repo
      run: git.repo-context
    - name: nextStep
      run: some.other-middleware
      config:
        branch: $(output:repo:branch)
        repoUrl: $(output:repo:remoteUrl)
```

In this example, the `repo-context` middleware is used to get information about the current Git repository. The branch name and remote URL are then used in the next step of the pipeline.

## Usage in Pipelines

The Git plugin is commonly used in the early stages of a pipeline to gather information about the repository. This information can then be used to make decisions in later stages, such as:

- Determining the version number based on the branch name
- Creating releases only on specific branches
- Including repository information in notifications

For a complete example of using the Git plugin in a pipeline, see the [Docker Deployment](./examples/docker-deployment.md) and [NuGet Release Pipeline](./examples/nuget-release.md) examples.

## Next Steps

- Learn about the [GitHub Plugin](./github.md) for GitHub-specific operations
- Explore the [Semantic Release Plugin](./semantic-release.md) for semantic versioning
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins