---
title: Docker Plugin
description: Documentation for the Docker plugin in Moonlit
---

# Docker Plugin

The Docker plugin provides integration with Docker. It allows you to build Docker images, push them to registries, and deploy them to target environments.

## Installation

To use the Docker plugin in your Moonlit pipeline, add it to the `plugins` section of your configuration file:

```yaml
plugins:
  - name: "docker"
    url: "nuget://Wolfware.Moonlit.Plugins.Docker/1.0.0"
    config:
      username: $(DOCKER_USERNAME)
      password: $(DOCKER_PASSWORD)
```

Note that the Docker plugin requires credentials to authenticate with Docker registries. You can set these credentials as environment variables and reference them in your configuration file.

## Middlewares

The Docker plugin provides the following middlewares:

### build

The `build` middleware builds a Docker image.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| dockerfile | string | Yes | - | The path to the Dockerfile |
| context | string | Yes | - | The build context directory |
| tags | array | No | - | An array of tags to apply to the image |

#### Outputs

This middleware does not produce any documented outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  build:
    - name: buildImage
      run: docker.build
      config:
        dockerfile: "./Dockerfile"
        context: "./"
        tags:
          - "mycompany/myapp:1.0.0"
          - "mycompany/myapp:latest"
```

### login

The `login` middleware logs in to a Docker registry.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| registry | string | No | "docker.io" | The URL of the Docker registry to log in to |

#### Outputs

This middleware does not produce any documented outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  publish:
    - name: login
      run: docker.login
      config:
        registry: "docker.io"
```

### push

The `push` middleware pushes a Docker image to a registry.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| image | string | Yes | - | The name of the image to push |
| tags | array | Yes | - | An array of tags to push |

#### Outputs

This middleware does not produce any documented outputs.

| Name | Type | Description |
|------|------|-------------|
| *None* | | |

#### Example

```yaml
stages:
  publish:
    - name: push
      run: docker.push
      config:
        image: "mycompany/myapp"
        tags:
          - "1.0.0"
          - "latest"
```

### deploy

The `deploy` middleware deploys a Docker image to a target environment.

#### Inputs

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| image | string | Yes | - | The image to deploy |
| environment | string | Yes | - | The name of the environment to deploy to |
| host | string | Yes | - | The host to deploy to |
| sshKey | string | Yes | - | The SSH key to use for connecting to the host |
| composeFile | string | Yes | - | The path to the Docker Compose file |

#### Outputs

| Name | Type | Description |
|------|------|-------------|
| environment | string | The name of the environment that was deployed to |

#### Example

```yaml
stages:
  deploy:
    - name: deployToProduction
      run: docker.deploy
      condition: $(output:repo:branch) == 'main'
      config:
        image: "mycompany/myapp:$(output:version:nextVersion)"
        environment: "production"
        host: $(DEPLOY_HOST)
        sshKey: $(DEPLOY_SSH_KEY)
        composeFile: "./docker-compose.yml"
```

## Usage in Pipelines

The Docker plugin is commonly used in deployment pipelines to:

1. Build Docker images with appropriate tags
2. Push images to Docker registries
3. Deploy images to target environments

These middlewares are typically used together to create a complete Docker deployment pipeline.

For a complete example of using the Docker plugin in a pipeline, see the [Docker Deployment](./examples/docker-deployment.md) example.

## Next Steps

- Learn about the [Git Plugin](./git.md) for Git repository operations
- Explore the [GitHub Plugin](./github.md) for GitHub API integration
- See the [Semantic Release Plugin](./semantic-release.md) for semantic versioning
- See the [Configuration](../guide/concepts/configuration.md) page for more information about configuring plugins