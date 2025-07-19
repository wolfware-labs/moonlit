using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Docker.Abstractions;
using Wolfware.Moonlit.Plugins.Docker.Configuration;
using Wolfware.Moonlit.Plugins.Docker.Constants;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Docker.Middlewares;

public sealed class BuildAndPush : ReleaseMiddleware<BuildAndPushConfiguration>
{
  private readonly ILogger<BuildAndPush> _logger;
  private readonly IDockerClient _dockerClient;

  public BuildAndPush(ILogger<BuildAndPush> logger, IDockerClient dockerClient)
  {
    _logger = logger;
    _dockerClient = dockerClient;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    BuildAndPushConfiguration configuration)
  {
    _logger.LogInformation("Starting Docker build process...");

    var commandArguments = new List<string> {"build"};

    var builder = configuration.Builder ??
                  Environment.GetEnvironmentVariable(PluginEnvironmentVariables.DockerBuildxBuilder);

    if (!string.IsNullOrWhiteSpace(builder))
    {
      commandArguments.Add($"--builder {builder}");
    }

    if (configuration.Tags.Length > 0)
    {
      commandArguments.AddRange(configuration.Tags.Select(tag => $"--tag {tag}"));
    }

    if (!string.IsNullOrWhiteSpace(configuration.File))
    {
      commandArguments.Add($"--file {configuration.File}");
    }

    if (configuration.BuildArgs.Length > 0)
    {
      commandArguments.AddRange(configuration.BuildArgs.Select(buildArg => $"--build-arg {buildArg}"));
    }

    if (configuration.Labels.Count > 0)
    {
      commandArguments.AddRange(configuration.Labels.Select(label => $"--label {label.Key}={label.Value}"));
    }

    if (configuration.Platforms.Length > 0)
    {
      commandArguments.Add($"--platform {string.Join(",", configuration.Platforms)}");
    }

    if (configuration.NoCache)
    {
      commandArguments.Add("--no-cache");
    }

    if (configuration.Pull)
    {
      commandArguments.Add("--pull");
    }

    commandArguments.Add(configuration.Push ? "--push" : "--load");

    commandArguments.Add(configuration.Context);

    await _dockerClient.Run("buildx", commandArguments.ToArray(), context.CancellationToken);
    _logger.LogInformation("Docker build process completed successfully.");
    return MiddlewareResult.Success();
  }
}
