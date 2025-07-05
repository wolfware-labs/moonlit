using Microsoft.Extensions.Logging;
using Semver;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;
using Wolfware.Moonlit.Plugins.SemanticRelease.Services;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class CalculateVersion : ReleaseMiddleware<CalculateVersion.Configuration>
{
  public sealed class Configuration
  {
    public string InitialVersion { get; set; } = "1.0.0";

    public string? BaseVersion { get; set; }

    public string Branch { get; set; } = string.Empty;

    public CommitMessage[] Commits { get; set; } = [];

    public CommitsAnalyzerConfiguration CommitRules = CommitsAnalyzerConfiguration.CreateDefault();

    public Dictionary<string, string> PrereleaseMappings { get; set; } = new();
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, Configuration configuration)
  {
    if (configuration.Commits.Length == 0)
    {
      context.Logger.LogWarning("No commits provided for version calculation.");
      return Task.FromResult(MiddlewareResult.Success());
    }

    context.Logger.LogInformation("Calculating next version based on {CommitCount} commits.",
      configuration.Commits.Length);

    var nextVersion = GetNextVersion(configuration, context.Logger);

    context.Logger.LogInformation("Next version calculated: {NextVersion}", nextVersion);

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("NextVersion", nextVersion.ToString());
      output.Add("IsPrerelease", nextVersion.IsPrerelease);
    }));
  }

  private SemVersion GetNextVersion(Configuration configuration, ILogger logger)
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

  private string? GetPrereleaseSuffix(Configuration configuration, ILogger logger)
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
