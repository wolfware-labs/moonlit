using Octokit;

namespace Wolfware.Moonlit.Plugins.Github.Abstractions;

public interface IGitHubContext
{
  Repository Repository { get; }

  Task<IReadOnlyList<PullRequest>> GetPullRequests(PullRequestRequest request);

  Task<IReadOnlyList<Issue>> GetIssues(RepositoryIssueRequest request);

  Task<Release> CreateRelease(NewRelease release);
}
