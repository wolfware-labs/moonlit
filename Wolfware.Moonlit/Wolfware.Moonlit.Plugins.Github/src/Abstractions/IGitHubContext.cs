using Octokit;

namespace Wolfware.Moonlit.Plugins.Github.Abstractions;

public interface IGitHubContext
{
  Repository Repository { get; }

  Task<IReadOnlyList<PullRequest>> GetPullRequests(PullRequestRequest request);

  Task<IReadOnlyList<Issue>> GetIssues(RepositoryIssueRequest request);

  Task<GitTag> CreateTag(string tag, string sha, string message);

  Task<Release> CreateRelease(NewRelease release);

  Task<IReadOnlyList<GitHubCommit>> GetCommits(CommitRequest request);

  Task<IReadOnlyList<Branch>> GetBranches(ApiOptions request);

  Task<IReadOnlyList<RepositoryTag>> GetTags(ApiOptions request);

  Task CommentOnPullRequest(int pullRequestNumber, string comment);

  Task LabelPullRequest(int pullRequestNumber, string label);

  Task CommentOnIssue(int issueNumber, string comment);

  Task LabelIssue(int issueNumber, string label);

  void SetOutput(string name, string value);

  void SetEnvironmentVariable(string name, string value);
}
