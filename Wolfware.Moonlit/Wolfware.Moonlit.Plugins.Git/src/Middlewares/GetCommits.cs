using LibGit2Sharp;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Git.Models;
using Wolfware.Moonlit.Plugins.Git.Services;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class GetCommits : ReleaseMiddleware<GetCommitsConfiguration>
{
  private readonly ILogger<GetCommits> _logger;
  private readonly SharedContext _sharedContext;

  public GetCommits(ILogger<GetCommits> logger, SharedContext sharedContext)
  {
    _logger = logger;
    _sharedContext = sharedContext;
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, GetCommitsConfiguration configuration)
  {
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();
    using var repo = new Repository(gitFolderPath);

    IEnumerable<Commit> commits = repo.Commits.OrderByDescending(c => c.Committer.When);
    var sinceSha = configuration.SinceSha;
    if (configuration.UseSharedContext && string.IsNullOrWhiteSpace(sinceSha))
    {
      sinceSha = this._sharedContext.LatestTagSha;
    }

    if (!string.IsNullOrWhiteSpace(sinceSha))
    {
      this._logger.LogInformation("Filtering commits since SHA: {SinceSha}", sinceSha[..7]);
      commits = commits.TakeWhile(c => !string.Equals(c.Sha, sinceSha, StringComparison.OrdinalIgnoreCase));
    }

    var commitDetails = commits.Select(c => new CommitDetails
    {
      Sha = c.Sha,
      Author = c.Author.Name,
      Email = c.Author.Email,
      Date = c.Author.When,
      Message = c.MessageShort
    }).ToArray();

    this._logger.LogInformation("Found {Count} commits", commitDetails.Length);

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("details", commitDetails);
      output.Add("count", commitDetails.Length);
    }));
  }
}
