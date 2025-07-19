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

    var name = string.IsNullOrWhiteSpace(configuration.Name) ? $"moonlit-builder-{Guid.NewGuid()}" : configuration.Name;
    var driver = string.IsNullOrWhiteSpace(configuration.Driver) ? "docker-container" : configuration.Driver;

    var commandArguments = new List<string> {"create", $"--name {name}", $"--driver {driver}"};

    if (configuration.Use)
    {
      commandArguments.Add("--use");
    }

    if (configuration.Bootstrap)
    {
      commandArguments.Add("--bootstrap");
    }

    if (configuration.Platforms.Length > 0)
    {
      commandArguments.Add($"--platform {string.Join(",", configuration.Platforms)}");
    }

    if (!string.IsNullOrWhiteSpace(configuration.Endpoint))
    {
      commandArguments.Add(configuration.Endpoint);
    }

    await _dockerClient.Run("buildx", commandArguments.ToArray(), context.CancellationToken);

    _logger.LogInformation("Docker Buildx setup process completed successfully.");
    return MiddlewareResult.Success();
  }
}
