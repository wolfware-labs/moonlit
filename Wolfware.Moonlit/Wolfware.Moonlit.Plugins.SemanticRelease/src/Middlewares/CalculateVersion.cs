using Microsoft.Extensions.Logging;
using Semver;
using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Services;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class CalculateVersion : ReleaseMiddleware<CalculateVersionConfiguration>
{
  private readonly ILogger<CalculateVersion> _logger;
  private readonly SharedContext _sharedContext;

  public CalculateVersion(ILogger<CalculateVersion> logger, SharedContext sharedContext)
  {
    _logger = logger;
    _sharedContext = sharedContext;
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    CalculateVersionConfiguration configuration)
  {
    configuration.Commits ??= this._sharedContext.Commits;
    if (configuration.Commits.Length == 0)
    {
      return Task.FromResult(MiddlewareResult.Failure("No commits provided for version calculation."));
    }

    this._logger.LogInformation("Calculating next version based on {CommitCount} commits.",
      configuration.Commits.Length);

    var nextVersion = GetNextVersion(configuration);
    if (nextVersion is null)
    {
      return Task.FromResult(MiddlewareResult.Success(output =>
      {
        output.Add("hasNewVersion", false);
      }));
    }

    this._logger.LogInformation("Next version calculated: {NextVersion}", nextVersion);

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("hasNewVersion", true);
      output.Add("nextVersion", nextVersion.WithoutMetadata().ToString());
      output.Add("nextFullVersion", nextVersion.ToString());
      output.Add("isPrerelease", nextVersion.IsPrerelease);
    }));
  }

  private SemVersion? GetNextVersion(CalculateVersionConfiguration configuration)
  {
    var prereleaseSuffix = GetPrereleaseSuffix(configuration);
    var metadata = $"sha-{configuration.Commits!.OrderBy(c => c.Date).Last().Sha[..7]}";
    if (string.IsNullOrWhiteSpace(configuration.BaseVersion))
    {
      return CalculateVersion.ParseInitialVersion(configuration.InitialVersion, prereleaseSuffix)
        .WithMetadata(metadata);
    }

    var analyzer = new ConventionalCommitsAnalyzer(configuration.ConventionalCommitRules);
    var calculator = new SemanticVersionCalculator(analyzer);
    var baseVersion = SemVersion.Parse(configuration.BaseVersion);
    return calculator.CalculateNextVersion(baseVersion, configuration.Commits!, prereleaseSuffix)
      ?.WithMetadata(metadata);
  }

  private string? GetPrereleaseSuffix(CalculateVersionConfiguration configuration)
  {
    if (!configuration.PrereleaseMappings.TryGetValue(configuration.Branch, out var prerelease))
    {
      return prerelease;
    }

    if (string.IsNullOrEmpty(prerelease))
    {
      return null;
    }

    this._logger.LogInformation("Using branch-specific prerelease mapping: {Prerelease}", prerelease);
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
