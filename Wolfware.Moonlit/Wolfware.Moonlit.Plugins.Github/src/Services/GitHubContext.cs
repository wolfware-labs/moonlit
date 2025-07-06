using Octokit;
using Wolfware.Moonlit.Plugins.Github.Abstractions;

namespace Wolfware.Moonlit.Plugins.Github.Services;

public sealed class GitHubContext : IGitHubContext
{
  private readonly IGitHubClient _gitHubClient;

  public GitHubContext(IGitHubClient gitHubClient, Repository repository)
  {
    _gitHubClient = gitHubClient;
    Repository = repository;
  }

  public Repository Repository { get; }

  public Task<IReadOnlyList<PullRequest>> GetPullRequests(PullRequestRequest request)
  {
    ArgumentNullException.ThrowIfNull(request, nameof(request));
    return _gitHubClient.PullRequest.GetAllForRepository(Repository.Id, request);
  }

  public Task<IReadOnlyList<Issue>> GetIssues(RepositoryIssueRequest request)
  {
    ArgumentNullException.ThrowIfNull(request, nameof(request));
    return _gitHubClient.Issue.GetAllForRepository(Repository.Id, request);
  }

  public Task<Release> CreateRelease(NewRelease release)
  {
    ArgumentNullException.ThrowIfNull(release, nameof(release));
    return _gitHubClient.Repository.Release.Create(Repository.Id, release);
  }

  public Task<IReadOnlyList<GitHubCommit>> GetCommits(CommitRequest request)
  {
    ArgumentNullException.ThrowIfNull(request, nameof(request));
    return _gitHubClient.Repository.Commit.GetAll(Repository.Id, request);
  }

  public Task<IReadOnlyList<Branch>> GetBranches(ApiOptions request)
  {
    ArgumentNullException.ThrowIfNull(request, nameof(request));
    return _gitHubClient.Repository.Branch.GetAll(Repository.Id, request);
  }

  public Task<IReadOnlyList<RepositoryTag>> GetTags(ApiOptions request)
  {
    ArgumentNullException.ThrowIfNull(request, nameof(request));
    return _gitHubClient.Repository.GetAllTags(Repository.Id, request);
  }

  public Task CommentOnPullRequest(int pullRequestNumber, string comment)
  {
    ArgumentNullException.ThrowIfNull(comment, nameof(comment));
    return _gitHubClient.Issue.Comment.Create(Repository.Id, pullRequestNumber, comment);
  }

  public Task CommentOnIssue(int issueNumber, string comment)
  {
    ArgumentNullException.ThrowIfNull(comment, nameof(comment));
    return _gitHubClient.Issue.Comment.Create(Repository.Id, issueNumber, comment);
  }
}
