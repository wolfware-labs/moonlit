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

    context.Logger.LogInformation("Calculating next version based on {CommitCount} commits.", config.Commits.Length);

    var nextVersion = GetNextVersion(config, context.Logger);

    context.Logger.LogInformation("Next version calculated: {NextVersion}", nextVersion);

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("NextVersion", nextVersion.ToString());
      output.Add("IsPrerelease", nextVersion.IsPrerelease);
    }));
  }

  private SemVersion GetNextVersion(CalculateVersionConfiguration configuration, ILogger logger)
  {
    var prereleaseSuffix = GetPrereleaseSuffix(configuration, logger);
    if (string.IsNullOrWhiteSpace(configuration.BaseVersion))
    {
      return CalculateVersion.ParseInitialVersion(configuration.InitialVersion, prereleaseSuffix);
    }

    var analyzer = new CommitsAnalyzer(configuration.CommitRules);
    var calculator = new SemanticVersionCalculator(analyzer);
    var baseVersion = SemVersion.Parse(configuration.BaseVersion);
    var commits = ConventionalCommitParser.Parse(configuration.Commits.Select(c => c.Message).ToArray());
    return calculator.CalculateNextVersion(baseVersion, commits, prereleaseSuffix);
  }

  private string? GetPrereleaseSuffix(CalculateVersionConfiguration configuration, ILogger logger)
  {
    if (!configuration.PrereleaseMappings.TryGetValue(configuration.Branch, out var prerelease))
    {
      return prerelease;
    }

    if (string.IsNullOrEmpty(prerelease))
    {
      return null;
    }

    logger.LogInformation("Using branch-specific prerelease mapping: {Prerelease}", prerelease);
    return prerelease;
  }

  private static SemVersion ParseInitialVersion(string initialVersion, string? suffix)
  {
    var parsedVersion = SemVersion.Parse(initialVersion);

    if (!string.IsNullOrEmpty(suffix))
    {
      parsedVersion = parsedVersion.WithPrerelease(suffix, "1");
    }

    return parsedVersion;
  }
}
