using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Semver;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Services;

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

    if(config.PrereleaseMappings.TryGetValue(config.Branch, out var prerelease))
    {
      context.Logger.LogInformation("Using branch-specific prerelease mapping: {Prerelease}", prerelease);
    }

    context.Logger.LogInformation("Calculating next version based on {CommitCount} commits.", config.Commits.Length);
    var calculator = new SemanticVersionCalculator(config.Release);
    var nextVersion = calculator.CalculateNextVersion(
      SemVersion.Parse(config.BaseVersion),
      config.Commits.Select(c => c.Message).ToArray(),
      prerelease
    );

    context.Logger.LogInformation("Next version calculated: {NextVersion}", nextVersion);

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("NextVersion", nextVersion.ToString());
      output.Add("IsPrerelease", nextVersion.IsPrerelease);
    }));
  }
}
