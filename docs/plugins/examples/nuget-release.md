---
title: NuGet Release Pipeline Example
description: A complete example of using Moonlit to automate a NuGet package release
---

# NuGet Release Pipeline Example

This page provides a complete example of using Moonlit to automate the release of a NuGet package. The pipeline analyzes the Git repository, calculates the next version using semantic versioning, creates a GitHub release, and notifies a Slack channel.

## Prerequisites

Before using this pipeline, ensure you have:

- A .NET project that you want to package as a NuGet package
- A Git repository hosted on GitHub
- A GitHub personal access token with appropriate permissions
- A Slack webhook or token (if using Slack notifications)
- OpenAI API key (for AI-generated changelogs)

## Configuration File

Here's the complete configuration file for the NuGet release pipeline:

```yaml
name: "Test Release"

plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
  - name: "slack"
    url: "nuget://Wolfware.Moonlit.Plugins.Slack/1.0.0"
    config:
      token: $(SLACK_TOKEN)

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
          main: next
    - name: changelog
      run: sr.generate-changelog
      config:
        commits: $(output:items:commits)
        openAiKey: $(OPENAI_API_KEY)

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

  notify:
    - name: notifySlackChannel
      run: "slack.send-notification"
      config:
        channel: "#moonlit"
        message: ":rocket:   New Release - <$(output:createRelease:url)|$(output:createRelease:name)>   :tada:"
```

## Pipeline Explanation

Let's break down this pipeline to understand how it works:

### Plugins

The pipeline uses four plugins:

1. **Git Plugin**: For Git repository operations
2. **GitHub Plugin**: For GitHub API integration
3. **Semantic Release Plugin**: For semantic versioning and changelog generation
4. **Slack Plugin**: For Slack notifications

Each plugin is configured with a name and URL, and some have additional configuration like tokens.

### Stages

The pipeline has three stages:

1. **analyze**: Gathers information about the repository and calculates the next version
2. **release**: Creates a GitHub release
3. **notify**: Sends a notification to a Slack channel

### Steps

#### Analyze Stage

1. **repo**: Gets information about the Git repository
   ```yaml
   - name: repo
     run: git.repo-context
   ```
   This step retrieves information about the current repository, such as the branch name, commit hash, and repository URL.

2. **tag**: Gets the latest tag from GitHub
   ```yaml
   - name: tag
     run: gh.latest-tag
     config:
       prefix: "v"
   ```
   This step retrieves the latest tag from GitHub that starts with "v" (e.g., "v1.0.0").

3. **items**: Gets commits, pull requests, and issues since the last tag
   ```yaml
   - name: items
     run: gh.items-since-commit
     config:
       tag: $(output:tag:commitSha)
   ```
   This step retrieves all commits, pull requests, and issues that have been created or merged since the last tag.

4. **version**: Calculates the next version using semantic versioning
   ```yaml
   - name: version
     run: sr.calculate-version
     config:
       branch: $(output:repo:branch)
       baseVersion: $(output:tag:name)
       commits: $(output:items:commits)
       prereleaseMappings:
         main: next
   ```
   This step calculates the next version based on the commit messages and the current branch. It uses the semantic versioning convention (major.minor.patch).

5. **changelog**: Generates a changelog from the commits
   ```yaml
   - name: changelog
     run: sr.generate-changelog
     config:
       commits: $(output:items:commits)
       openAiKey: $(OPENAI_API_KEY)
   ```
   This step generates a changelog from the commit messages. It uses OpenAI to generate more readable and comprehensive changelog entries.

#### Release Stage

1. **createRelease**: Creates a GitHub release
   ```yaml
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
   ```
   This step creates a GitHub release with the calculated version, changelog, and links to related pull requests and issues.

#### Notify Stage

1. **notifySlackChannel**: Sends a notification to a Slack channel
   ```yaml
   - name: notifySlackChannel
     run: "slack.send-notification"
     config:
       channel: "#moonlit"
       message: ":rocket:   New Release - <$(output:createRelease:url)|$(output:createRelease:name)>   :tada:"
   ```
   This step sends a notification to a Slack channel with a link to the GitHub release.

## Running the Pipeline

To run this pipeline, save the configuration to a file (e.g., `moonlit.yml`) and run:

```bash
# Set environment variables
set GITHUB_TOKEN=your_github_token
set SLACK_TOKEN=your_slack_token
set OPENAI_API_KEY=your_openai_api_key

# Run the pipeline
moonlit -f moonlit.yml
```

## Extending the Pipeline

You can extend this pipeline to include additional steps, such as:

- Building the project
- Running tests
- Packing the NuGet package
- Publishing the package to NuGet.org

Here's an example of how you might add these steps:

```yaml
plugins:
  # ... existing plugins ...
  - name: "dotnet"
    url: "nuget://Wolfware.Moonlit.Plugins.Dotnet/1.0.0"
    config:
      apiKey: $(NUGET_API_KEY)

stages:
  # ... existing stages ...

  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        configuration: "Release"

    - name: test
      run: dotnet.test
      config:
        project: "./tests/MyProject.Tests.csproj"

  publish:
    - name: pack
      run: dotnet.pack
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextVersion)

    - name: push
      run: dotnet.push
      config:
        package: $(output:pack:packagePath)
        source: "https://api.nuget.org/v3/index.json"
```

## Next Steps

- Learn about the [Docker Deployment](./docker-deployment.md) example
- Explore the [available plugins](../index.md)
- See how to [create your own plugins](../../reference/plugin-development.md)
