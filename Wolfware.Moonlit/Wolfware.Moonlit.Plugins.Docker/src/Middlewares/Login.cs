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
    this._logger = logger;
    this._dockerClient = dockerClient;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, LoginConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(configuration.Username) || string.IsNullOrWhiteSpace(configuration.Password))
    {
      return MiddlewareResult.Failure("Docker login requires both username and password to be set.");
    }

    this._logger.LogInformation("Starting Docker login process...");
    this._logger.LogInformation("Logging in to Docker registry {Server} with user {Username}",
      configuration.Server ?? "Docker Hub",
      configuration.Username);
    var commandOptions = new[]
    {
      new KeyValuePair<string, string>("--username", configuration.Username),
      new KeyValuePair<string, string>("--password", configuration.Password)
    };
    var commandArgs = configuration.Server != null ? new[] {configuration.Server} : Array.Empty<string>();
    await this._dockerClient.RunDockerCommand("login", commandArgs, commandOptions);
    this._logger.LogInformation("Docker login process completed successfully.");
    return MiddlewareResult.Success();
  }
}
