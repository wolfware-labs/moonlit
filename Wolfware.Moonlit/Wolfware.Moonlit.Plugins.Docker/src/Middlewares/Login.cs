using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Docker.Abstractions;
using Wolfware.Moonlit.Plugins.Docker.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Docker.Middlewares;

public sealed class Login : ReleaseMiddleware<LoginConfiguration>
{
  private readonly ILogger<Login> _logger;
  private readonly IDockerClient _dockerClient;

  public Login(ILogger<Login> logger, IDockerClient dockerClient)
  {
    _logger = logger;
    _dockerClient = dockerClient;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, LoginConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(configuration.Username) || string.IsNullOrWhiteSpace(configuration.Password))
    {
      return MiddlewareResult.Failure("Docker login requires both username and password to be set.");
    }

    _logger.LogInformation("Starting Docker login process...");
    await _dockerClient.Login(configuration.Server, configuration.Username, configuration.Password);
    _logger.LogInformation("Docker login process completed successfully.");
    return MiddlewareResult.Success();
  }
}
