using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Github.Models;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class GetRelatedItems : ReleaseMiddleware<GetRelatedItemsConfiguration>
{
  private readonly IGitHubContextProvider _contextProvider;
  private readonly ILogger<GetRelatedItems> _logger;
  private IGitHubContext? _githubContext;

  public GetRelatedItems(IGitHubContextProvider contextProvider, ILogger<GetRelatedItems> logger)
  {
    _contextProvider = contextProvider;
    _logger = logger;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(
    ReleaseContext context,
    GetRelatedItemsConfiguration configuration
  )
  {
    if (configuration.Commits.Length == 0)
    {
      this._logger.LogInformation("No commits provided in configuration");
      return MiddlewareResult.Success();
    }

    this._githubContext = await _contextProvider.GetCurrentContext(context);

    PullRequestDetails[]? pullRequests = null;
    if (configuration.IncludePullRequests || configuration.IncludeIssues)
    {
      this._logger.LogInformation("Fetching pull requests from commits");
      var commitShas = configuration.Commits.Select(c => c.Sha).ToArray();
      pullRequests = await this.GetPullRequestsFromCommits(commitShas).ConfigureAwait(false);
      this._logger.LogInformation("Found {Count} pull requests", pullRequests.Length);
    }

    IssueDetails[]? issues = null;
    if (configuration.IncludeIssues && pullRequests is {Length: > 0})
    {
      this._logger.LogInformation("Fetching issues from pull requests");
      var pullRequestNumbers = pullRequests.Select(pr => pr.Number).ToArray();
      issues = await this.GetIssuesFromPullRequests(pullRequestNumbers).ConfigureAwait(false);
      this._logger.LogInformation("Found {Count} issues", issues.Length);
    }

    return MiddlewareResult.Success(output =>
    {
      if (pullRequests is not null && pullRequests.Length > 0)
      {
        output.Add("prs", pullRequests);
      }

      if (issues is not null && issues.Length > 0)
      {
        output.Add("issues", issues);
      }
    });
  }

  private async Task<PullRequestDetails[]> GetPullRequestsFromCommits(string[] commits)
  {
    var prs = await this._githubContext!.GetPullRequests(new PullRequestRequest {State = ItemStateFilter.All});
    if (prs.Count == 0)
    {
      return [];
    }

    return prs
      .Where(pr => pr.MergeCommitSha != null && commits.Any(c => c == pr.MergeCommitSha))
      .Select(pr => new PullRequestDetails
      {
        Number = pr.Number,
        Title = pr.Title,
        Body = pr.Body,
        State = pr.State.Value,
        CreatedAt = pr.CreatedAt,
        UpdatedAt = pr.UpdatedAt,
        MergedAt = pr.MergedAt,
        MergeCommitSha = pr.MergeCommitSha
      })
      .OrderByDescending(pr => pr.CreatedAt)
      .ToArray();
  }

  private async Task<IssueDetails[]> GetIssuesFromPullRequests(int[] pullRequestNumbers)
  {
    var issues = await this._githubContext!.GetIssues(new RepositoryIssueRequest {State = ItemStateFilter.All});
    if (issues.Count == 0)
    {
      return [];
    }

    return issues
      .Where(issue => issue.PullRequest != null && pullRequestNumbers.Any(pr => pr == issue.PullRequest.Number))
      .Select(issue => new IssueDetails
      {
        Number = issue.Number,
        Title = issue.Title,
        Body = issue.Body,
        State = issue.State.Value,
        CreatedAt = issue.CreatedAt,
        UpdatedAt = issue.UpdatedAt,
        ClosedAt = issue.ClosedAt,
        PullRequestNumber = issue.PullRequest?.Number ?? 0
      })
      .OrderByDescending(issue => issue.CreatedAt)
      .ToArray();
  }
}
