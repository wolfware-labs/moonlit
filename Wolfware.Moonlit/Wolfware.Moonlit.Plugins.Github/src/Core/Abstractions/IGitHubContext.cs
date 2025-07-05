using Octokit;

namespace Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

public interface IGitHubContext
{
  string CurrentBranch { get; }

  Repository Repository { get; }

  Task<IReadOnlyList<PullRequest>> GetPullRequests(PullRequestRequest request);

  Task<IReadOnlyList<Issue>> GetIssues(RepositoryIssueRequest request);

  Task<Release> CreateRelease(NewRelease release);

  Task<IReadOnlyList<GitHubCommit>> GetCommits(CommitRequest request);

  Task<IReadOnlyList<Branch>> GetBranches(ApiOptions request);

  Task<IReadOnlyList<RepositoryTag>> GetTags(ApiOptions request);
}
