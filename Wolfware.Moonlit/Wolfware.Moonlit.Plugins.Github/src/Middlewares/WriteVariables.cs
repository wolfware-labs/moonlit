using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class WriteVariables : ReleaseMiddleware<WriteVariablesConfiguration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;
  private readonly ILogger<WriteVariables> _logger;

  public WriteVariables(IGitHubContextProvider gitHubContextProvider, ILogger<WriteVariables> logger)
  {
    _gitHubContextProvider = gitHubContextProvider;
    _logger = logger;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    WriteVariablesConfiguration configuration)
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);
    foreach ((var key, var value) in configuration.Output)
    {
      this._logger.LogInformation("Setting output variable '{Key}' to '{Value}'", key, value);
      gitHubContext.SetOutput(key, value);
    }

    foreach ((var key, var value) in configuration.Environment)
    {
      this._logger.LogInformation("Setting environment variable '{Key}' to '{Value}'", key, value);
      gitHubContext.SetEnvironmentVariable(key, value);
    }

    return MiddlewareResult.Success();
  }
}
