using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Services;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class GenerateChangelog : ReleaseMiddleware<GenerateChangelogConfiguration>
{
  private readonly ILogger<GenerateChangelog> _logger;
  private readonly SharedContext _sharedContext;
  private readonly IAiAgent _aiAgent;
  private readonly IChangelogGenerator _changelogGenerator;

  public GenerateChangelog(ILogger<GenerateChangelog> logger, SharedContext sharedContext, IAiAgent aiAgent,
    IChangelogGenerator changelogGenerator)
  {
    this._logger = logger;
    this._sharedContext = sharedContext;
    this._aiAgent = aiAgent;
    this._changelogGenerator = changelogGenerator;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    GenerateChangelogConfiguration configuration)
  {
    configuration.Commits ??= this._sharedContext.Commits;
    if (configuration.Commits.Length == 0)
    {
      this._logger.LogWarning("No commits provided for changelog generation.");
      return MiddlewareResult.Success();
    }

    this._logger.LogInformation("Generating changelog for {CommitCount} commits.", configuration.Commits.Length);

    try
    {
      var commits = configuration.Commits;

      if (configuration.FilterNonUserFacingCommits)
      {
        commits = await _aiAgent.FilterOutNonUserFacingCommits(commits);
        if (commits.Length == 0)
        {
          this._logger.LogInformation("No user-facing commits found. Skipping changelog generation.");
          return MiddlewareResult.Success();
        }

        if (commits.Length < configuration.Commits.Length)
        {
          this._logger.LogInformation("Filtered down to {UserFacingCommitCount} user-facing commits.",
            commits.Length);
        }
      }

      if (configuration.RefineCommitsSummary)
      {
        commits = await _aiAgent.RefineCommitsSummary(commits);
      }

      var categories = this._changelogGenerator.GenerateChangelog(commits, configuration.ChangelogRules);

      return MiddlewareResult.Success(output =>
      {
        output.Add("categories", categories);
      });
    }
    catch (Exception ex)
    {
      this._logger.LogError(ex, "An error occurred while generating changelogs.");
      return MiddlewareResult.Failure(ex.Message);
    }
  }
}
