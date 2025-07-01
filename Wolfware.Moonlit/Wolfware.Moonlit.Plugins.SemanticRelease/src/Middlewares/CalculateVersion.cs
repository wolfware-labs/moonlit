using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Semver;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class CalculateVersion : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context);
    ArgumentNullException.ThrowIfNull(configuration);

    var config = configuration.GetRequired<CalculateVersionConfiguration>();
    if (config.Commits.Length == 0)
    {
      context.Logger.LogWarning("No commits provided for version calculation.");
      return Task.FromResult(MiddlewareResult.Success());
    }

    var calculator = new SemanticVersionCalculator(config.Release);
    var nextVersion = calculator.CalculateNextVersion(SemVersion.Parse(config.BaseVersion), config.Commits);
    return Task.FromResult(MiddlewareResult.Success());
  }
}
