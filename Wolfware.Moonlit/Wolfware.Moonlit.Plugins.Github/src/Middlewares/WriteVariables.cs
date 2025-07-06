using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class WriteVariables : ReleaseMiddleware<WriteVariablesConfiguration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public WriteVariables(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    WriteVariablesConfiguration configuration)
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);
    foreach ((var key, var value) in configuration.Output)
    {
      context.Logger.LogInformation("Setting output variable '{Key}' to '{Value}'", key, value);
      gitHubContext.SetOutput(key, value);
    }

    foreach ((var key, var value) in configuration.Environment)
    {
      context.Logger.LogInformation("Setting environment variable '{Key}' to '{Value}'", key, value);
      gitHubContext.SetEnvironmentVariable(key, value);
    }

    return MiddlewareResult.Success();
  }
}
