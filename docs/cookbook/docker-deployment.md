---
title: Docker Deployment Example
description: A complete example of using Moonlit to automate Docker image building and deployment
---

# Docker Deployment Example

This page provides a complete example of using Moonlit to automate the building and deployment of a Docker image. The pipeline builds a Docker image, tags it with a semantic version, pushes it to a registry, and deploys it to a target environment.

## Prerequisites

Before using this pipeline, ensure you have:

- A project with a Dockerfile
- Access to a Docker registry (Docker Hub, GitHub Container Registry, etc.)
- Docker installed on the machine running Moonlit
- Appropriate credentials for your Docker registry

## Configuration File

Here's the complete configuration file for the Docker deployment pipeline:

```yaml
name: "Docker Deployment"

plugins:
  - name: "git"
    url: "nuget://Wolfware.Moonlit.Plugins.Git/1.0.0"
  - name: "gh"
    url: "nuget://Wolfware.Moonlit.Plugins.Github/1.0.0"
    config:
      token: $(GITHUB_TOKEN)
  - name: "sr"
    url: "nuget://Wolfware.Moonlit.Plugins.SemanticRelease/1.0.0"
  - name: "docker"
    url: "nuget://Wolfware.Moonlit.Plugins.Docker/1.0.0"
    config:
      username: $(DOCKER_USERNAME)
      password: $(DOCKER_PASSWORD)
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
          main: latest
          develop: staging
          feature/*: dev

  build:
    - name: buildImage
      run: docker.build
      config:
        dockerfile: "./Dockerfile"
        context: "./"
        tags:
          - "mycompany/myapp:$(output:version:nextVersion)"
          - "mycompany/myapp:$(output:version:prereleaseName)"

  publish:
    - name: login
      run: docker.login
      config:
        registry: "docker.io"
    - name: push
      run: docker.push
      config:
        image: "mycompany/myapp"
        tags:
          - "$(output:version:nextVersion)"
          - "$(output:version:prereleaseName)"

  deploy:
    - name: deployToEnvironment
      run: docker.deploy
      condition: $(output:repo:branch) == 'main'
      config:
        image: "mycompany/myapp:$(output:version:nextVersion)"
        environment: "production"
        host: $(DEPLOY_HOST)
        sshKey: $(DEPLOY_SSH_KEY)
        composeFile: "./docker-compose.yml"

  notify:
    - name: notifySlackChannel
      run: "slack.send-notification"
      config:
        channel: "#deployments"
        message: ":whale:   New Docker Deployment - `mycompany/myapp:$(output:version:nextVersion)` deployed to $(output:deployToEnvironment:environment)   :rocket:"
```

## Pipeline Explanation

Let's break down this pipeline to understand how it works:

### Plugins

The pipeline uses five plugins:

1. **Git Plugin**: For Git repository operations
2. **GitHub Plugin**: For GitHub API integration
3. **Semantic Release Plugin**: For semantic versioning
4. **Docker Plugin**: For Docker operations
5. **Slack Plugin**: For Slack notifications

Each plugin is configured with a name and URL, and some have additional configuration like tokens and credentials.

### Stages

The pipeline has five stages:

1. **analyze**: Gathers information about the repository and calculates the next version
2. **build**: Builds the Docker image
3. **publish**: Pushes the Docker image to a registry
4. **deploy**: Deploys the Docker image to a target environment
5. **notify**: Sends a notification to a Slack channel

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

3. **items**: Gets commits since the last tag
   ```yaml
   - name: items
     run: gh.items-since-commit
     config:
       tag: $(output:tag:commitSha)
   ```
   This step retrieves all commits that have been created or merged since the last tag.

4. **version**: Calculates the next version using semantic versioning
   ```yaml
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
   ```
   This step calculates the next version based on the commit messages and the current branch. It also maps branch names to prerelease identifiers.

#### Build Stage

1. **buildImage**: Builds the Docker image
   ```yaml
   - name: buildImage
     run: docker.build
     config:
       dockerfile: "./Dockerfile"
       context: "./"
       tags:
         - "mycompany/myapp:$(output:version:nextVersion)"
         - "mycompany/myapp:$(output:version:prereleaseName)"
   ```
   This step builds a Docker image using the specified Dockerfile and context. It tags the image with the calculated version and a prerelease name based on the branch.

#### Publish Stage

1. **login**: Logs in to the Docker registry
   ```yaml
   - name: login
     run: docker.login
     config:
       registry: "docker.io"
   ```
   This step logs in to the Docker registry using the credentials provided in the plugin configuration.

2. **push**: Pushes the Docker image to the registry
   ```yaml
   - name: push
     run: docker.push
     config:
       image: "mycompany/myapp"
       tags:
         - "$(output:version:nextVersion)"
         - "$(output:version:prereleaseName)"
   ```
   This step pushes the Docker image to the registry with the specified tags.

#### Deploy Stage

1. **deployToEnvironment**: Deploys the Docker image to a target environment
   ```yaml
   - name: deployToEnvironment
     run: docker.deploy
     condition: $(output:repo:branch) == 'main'
     config:
       image: "mycompany/myapp:$(output:version:nextVersion)"
       environment: "production"
       host: $(DEPLOY_HOST)
       sshKey: $(DEPLOY_SSH_KEY)
       composeFile: "./docker-compose.yml"
   ```
   This step deploys the Docker image to a target environment using Docker Compose. It only runs if the current branch is 'main'.

#### Notify Stage

1. **notifySlackChannel**: Sends a notification to a Slack channel
   ```yaml
   - name: notifySlackChannel
     run: "slack.send-notification"
     config:
       channel: "#deployments"
       message: ":whale:   New Docker Deployment - `mycompany/myapp:$(output:version:nextVersion)` deployed to $(output:deployToEnvironment:environment)   :rocket:"
   ```
   This step sends a notification to a Slack channel with information about the deployment.

## Running the Pipeline

To run this pipeline, save the configuration to a file (e.g., `moonlit.yml`) and run:

```bash
# Set environment variables
set GITHUB_TOKEN=your_github_token
set DOCKER_USERNAME=your_docker_username
set DOCKER_PASSWORD=your_docker_password
set DEPLOY_HOST=your_deploy_host
set DEPLOY_SSH_KEY=your_deploy_ssh_key
set SLACK_TOKEN=your_slack_token

# Run the pipeline
moonlit -f moonlit.yml
```

## Customizing the Pipeline

You can customize this pipeline for your specific needs:

- Change the Docker image name and tags
- Modify the deployment configuration
- Add additional build steps
- Configure different environments based on branches

For example, you might want to deploy to different environments based on the branch:

```yaml
stages:
  # ... existing stages ...

  deploy:
    - name: deployToProduction
      run: docker.deploy
      condition: $(output:repo:branch) == 'main'
      config:
        image: "mycompany/myapp:$(output:version:nextVersion)"
        environment: "production"
        host: $(DEPLOY_PROD_HOST)
        sshKey: $(DEPLOY_SSH_KEY)
        composeFile: "./docker-compose.prod.yml"
    
    - name: deployToStaging
      run: docker.deploy
      condition: $(output:repo:branch) == 'develop'
      config:
        image: "mycompany/myapp:$(output:version:prereleaseName)"
        environment: "staging"
        host: $(DEPLOY_STAGING_HOST)
        sshKey: $(DEPLOY_SSH_KEY)
        composeFile: "./docker-compose.staging.yml"
```

## Next Steps

- Learn about the [NuGet Release Pipeline](./nuget-release) example
- Explore the [available plugins](/plugins/)
- See how to [create your own plugins](/reference/plugin-development)