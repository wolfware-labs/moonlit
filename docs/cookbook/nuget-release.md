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
name: "NuGet Package Release"

plugins:
  - name: "git"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Git/1.0.0-next.5"
  - name: "gh"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Github/1.0.0-next.6"
    config:
      token: $(GITHUB_TOKEN)
  - name: "sr"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0-next.5"
    config:
      openAi:
        apiKey: $(OPENAI_API_KEY)
  - name: "dotnet"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Dotnet/1.0.0-next.5"
    config:
      nugetApiKey: $(NUGET_API_KEY)
  - name: "slack"
    url: "nuget://nuget.org/Wolfware.Moonlit.Plugins.Slack/1.0.0-next.5"
    config:
      token: $(SLACK_TOKEN)

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
    - name: ghItems
      run: gh.related-items
      config:
        commits: $(output:commits:details)
    - name: conventionalCommits
      run: sr.analyze
      haltIf: output.conventionalCommits.commitCount == 0
      config:
        commits: $(output:commits:details)
        includeScopes:
          - myproject
    - name: version
      run: sr.calculate-version
      haltIf: "!output.version.hasNewVersion"
      config:
        branch: $(output:repo:branch)
        baseVersion: $(output:tag:name)
        prereleaseMappings:
          main: next
    - name: changelog
      run: sr.generate-changelog

  build:
    - name: build
      run: dotnet.build
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextFullVersion)
        configuration: "Release"
    - name: pack
      run: dotnet.pack
      config:
        project: "./src/MyProject.csproj"
        version: $(output:version:nextFullVersion)

  release:
    - name: publish
      run: dotnet.push
      config:
        package: $(output:pack:packagePath)
    - name: createRelease
      run: gh.create-release
      config:
        name: "Release $(output:version:nextVersion)"
        tag: v$(output:version:nextVersion)
        label: "released on @$(output:repo:branch)"
        changelog: $(output:changelog:categories)
        prerelease: $(output:version:isPrerelease)
        pullRequests: $(output:ghItems:pullRequests)
        issues: $(output:ghItems:issues)

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

The pipeline uses five plugins:

1. **Git Plugin**: For Git repository operations
2. **GitHub Plugin**: For GitHub API integration
3. **Semantic Release Plugin**: For semantic versioning and changelog generation
4. **Dotnet Plugin**: For building, packing, and publishing .NET projects
5. **Slack Plugin**: For Slack notifications

Each plugin is configured with a name and URL, and some have additional configuration like tokens and API keys.

### Stages

The pipeline has four stages:

1. **analyze**: Gathers information about the repository and calculates the next version
2. **build**: Builds and packages the .NET project
3. **release**: Publishes the package and creates a GitHub release
4. **notify**: Sends a notification to a Slack channel

### Steps

#### Analyze Stage

1. **repo**: Gets information about the Git repository
   ```yaml
   - name: repo
     run: git.repo-context
   ```
   This step retrieves information about the current repository, such as the branch name, commit hash, and repository URL.

2. **tag**: Gets the latest tag from the Git repository
   ```yaml
   - name: tag
     run: git.latest-tag
     config:
       prefix: "v"
   ```
   This step retrieves the latest tag from the Git repository that starts with "v" (e.g., "v1.0.0").

3. **commits**: Gets commits since the last tag
   ```yaml
   - name: commits
     run: git.commits
   ```
   This step retrieves all commits that have been created since the last tag.

4. **ghItems**: Gets pull requests and issues related to the commits
   ```yaml
   - name: ghItems
     run: gh.related-items
     config:
       commits: $(output:commits:details)
   ```
   This step retrieves all pull requests and issues that are related to the commits.

5. **conventionalCommits**: Analyzes commits for conventional commit format
   ```yaml
   - name: conventionalCommits
     run: sr.analyze
     haltIf: output.conventionalCommits.commitCount == 0
     config:
       commits: $(output:commits:details)
       includeScopes:
         - myproject
   ```
   This step analyzes the commits to identify conventional commits and categorize them by type. It halts the pipeline if no conventional commits are found.

6. **version**: Calculates the next version using semantic versioning
   ```yaml
   - name: version
     run: sr.calculate-version
     haltIf: "!output.version.hasNewVersion"
     config:
       branch: $(output:repo:branch)
       baseVersion: $(output:tag:name)
       prereleaseMappings:
         main: next
   ```
   This step calculates the next version based on the conventional commits and the current branch. It halts the pipeline if no new version is needed.

7. **changelog**: Generates a changelog from the conventional commits
   ```yaml
   - name: changelog
     run: sr.generate-changelog
   ```
   This step generates a changelog from the conventional commits, organized by category (features, fixes, etc.).

#### Build Stage

1. **build**: Builds the .NET project
   ```yaml
   - name: build
     run: dotnet.build
     config:
       project: "./src/MyProject.csproj"
       version: $(output:version:nextFullVersion)
       configuration: "Release"
   ```
   This step builds the .NET project with the calculated version.

2. **pack**: Creates a NuGet package
   ```yaml
   - name: pack
     run: dotnet.pack
     config:
       project: "./src/MyProject.csproj"
       version: $(output:version:nextFullVersion)
   ```
   This step creates a NuGet package with the calculated version.

#### Release Stage

1. **publish**: Publishes the NuGet package
   ```yaml
   - name: publish
     run: dotnet.push
     config:
       package: $(output:pack:packagePath)
   ```
   This step publishes the NuGet package to the configured feed.

2. **createRelease**: Creates a GitHub release
   ```yaml
   - name: createRelease
     run: gh.create-release
     config:
       name: "Release $(output:version:nextVersion)"
       tag: v$(output:version:nextVersion)
       label: "released on @$(output:repo:branch)"
       changelog: $(output:changelog:categories)
       prerelease: $(output:version:isPrerelease)
       pullRequests: $(output:ghItems:pullRequests)
       issues: $(output:ghItems:issues)
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
set NUGET_API_KEY=your_nuget_api_key

# Run the pipeline
moonlit -f moonlit.yml
```

## Extending the Pipeline

You can extend this pipeline to include additional steps, such as:

- Running tests before building
- Signing the NuGet package
- Publishing symbols to a symbol server
- Deploying documentation
- Creating GitHub deployment environments

Here's an example of how you might add tests to the pipeline:

```yaml
stages:
  # ... existing analyze stage ...

  test:
    - name: test
      run: dotnet.test
      config:
        project: "./tests/MyProject.Tests.csproj"
        configuration: "Release"
        
  # ... existing build, release, and notify stages ...
```

## Next Steps

- Learn about the [Docker Deployment](./docker-deployment) example
- Explore the [available plugins](/plugins/)
- See how to [create your own plugins](/reference/plugin-development)