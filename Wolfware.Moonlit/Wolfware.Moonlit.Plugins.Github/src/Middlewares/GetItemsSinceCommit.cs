using Octokit;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Github.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class GetItemsSinceCommit : ReleaseMiddleware<GetItemsSinceCommitConfiguration>
{
  private readonly IGitHubContextProvider _contextProvider;
  private IGitHubContext? _githubContext;

  public GetItemsSinceCommit(IGitHubContextProvider contextProvider)
  {
    _contextProvider = contextProvider;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(
    ReleaseContext context,
    GetItemsSinceCommitConfiguration configuration
  )
  {
    if (string.IsNullOrWhiteSpace(configuration.Commit))
    {
      return MiddlewareResult.Failure("Commit cannot be null or empty.");
    }

    this._githubContext = await _contextProvider.GetCurrentContext(context);

    CommitDetails[]? commits = null;
    if (configuration.IncludeCommits || configuration.IncludePullRequests || configuration.IncludeIssues)
    {
      commits = await this.GetCommitsSinceCommit(configuration.Commit).ConfigureAwait(false);
    }

    PullRequestDetails[]? pullRequests = null;
    if (configuration.IncludePullRequests || configuration.IncludeIssues)
    {
      var commitShas = commits?.Select(c => c.Sha).ToArray() ?? [];
      pullRequests = await this.GetPullRequestsFromCommits(commitShas).ConfigureAwait(false);
    }

    IssueDetails[]? issues = null;
    if (configuration.IncludeIssues && pullRequests is {Length: > 0})
    {
      var pullRequestNumbers = pullRequests.Select(pr => pr.Number).ToArray();
      issues = await this.GetIssuesFromPullRequests(pullRequestNumbers).ConfigureAwait(false);
    }

    return MiddlewareResult.Success(output =>
    {
      if (commits is not null && commits.Length > 0)
      {
        output.Add("Commits", commits);
      }

      if (pullRequests is not null && pullRequests.Length > 0)
      {
        output.Add("PullRequests", pullRequests);
      }

      if (issues is not null && issues.Length > 0)
      {
        output.Add("Issues", issues);
      }
    });
  }

  private async Task<CommitDetails[]> GetCommitsSinceCommit(string commit)
  {
    var commits = await this._githubContext!.GetCommits(new CommitRequest {Sha = commit});
    return commits.Select(c => new CommitDetails
      {
        Sha = c.Sha,
        Message = c.Commit.Message,
        Author = c.Author?.Login ?? c.Commit.Author.Name,
        Date = c.Commit.Author.Date
      }
    ).ToArray();
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
      .ToArray();
  }
}
