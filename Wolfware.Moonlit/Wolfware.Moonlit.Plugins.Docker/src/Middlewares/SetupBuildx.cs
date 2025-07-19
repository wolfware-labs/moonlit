using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Docker.Abstractions;
using Wolfware.Moonlit.Plugins.Docker.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Docker.Middlewares;

public sealed class SetupBuildx : ReleaseMiddleware<SetupBuildxConfiguration>
{
  private readonly ILogger<SetupBuildx> _logger;
  private readonly IDockerClient _dockerClient;

  public SetupBuildx(ILogger<SetupBuildx> logger, IDockerClient dockerClient)
  {
    _logger = logger;
    _dockerClient = dockerClient;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    SetupBuildxConfiguration configuration)
  {
    _logger.LogInformation("Starting Docker Buildx setup process...");

    string driver = string.IsNullOrWhiteSpace(configuration.Driver) ? "docker-container" : configuration.Driver;
    string? endpoint = string.IsNullOrWhiteSpace(configuration.Endpoint) ? null : configuration.Endpoint;

    _logger.LogInformation("Creating Docker Buildx builder with driver {Driver} and endpoint {Endpoint}",
      driver, endpoint ?? "default");

    var commandOptions = new List<KeyValuePair<string, string>> {new("--driver", driver), new("--use", string.Empty)};

    if (endpoint != null)
    {
      commandOptions.Add(new KeyValuePair<string, string>("--driver-opt", $"env.DOCKER_HOST={endpoint}"));
    }

    var commandArgs = new[] {configuration.BuilderName ?? "moonlit-builder"};

    await _dockerClient.RunDockerCommand("buildx", new[] {"create"}.Concat(commandArgs).ToArray(),
      commandOptions.ToArray());

    _logger.LogInformation("Docker Buildx setup process completed successfully.");
    return MiddlewareResult.Success();
  }
}
