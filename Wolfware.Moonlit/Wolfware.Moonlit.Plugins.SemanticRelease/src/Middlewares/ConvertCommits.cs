using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;
using Wolfware.Moonlit.Plugins.SemanticRelease.Services;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class ConvertCommits : ReleaseMiddleware<ConvertCommitsConfiguration>
{
  private readonly SharedContext _sharedContext;

  public ConvertCommits(SharedContext sharedContext)
  {
    _sharedContext = sharedContext;
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    ConvertCommitsConfiguration configuration)
  {
    var commits = ConventionalCommitConverter.Convert(configuration.Commits);
    commits = commits.Where(c => ConvertCommits.CheckCommit(c, configuration)).ToArray();
    this._sharedContext.Commits = commits;
    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("commits", commits);
      output.Add("commitCount", commits.Length);
    }));
  }

  private static bool CheckCommit(ConventionalCommit commit, ConvertCommitsConfiguration configuration)
  {
    if (commit.Scope is null)
    {
      return configuration.IncludeUnscoped;
    }

    if (configuration.IncludeScopes?.Length > 0)
    {
      return configuration.IncludeScopes.Contains(commit.Scope);
    }

    if (configuration.ExcludeScopes?.Length > 0)
    {
      return !configuration.ExcludeScopes.Contains(commit.Scope);
    }

    return true;
  }
}
