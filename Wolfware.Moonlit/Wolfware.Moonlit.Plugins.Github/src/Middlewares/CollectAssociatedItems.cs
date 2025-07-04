using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class CollectAssociatedItems : ReleaseMiddleware<CollectAssociatedItems.Configuration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public sealed class Configuration
  {
    public string[] Commits { get; set; } = [];

    public bool IncludePullRequests { get; set; } = true;

    public bool IncludeIssues { get; set; } = true;
  }

  public CollectAssociatedItems(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, Configuration configuration)
  {
    if (configuration.Commits.Length == 0)
    {
      context.Logger.LogWarning("No commits provided for associated items collection.");
      return MiddlewareResult.Success();
    }

    context.Logger.LogInformation("Collecting associated items for {CommitCount} commits.",
      configuration.Commits.Length);

    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);

    foreach (var commitSha in configuration.Commits)
    {
      context.Logger.LogInformation("Processing commit: {CommitSha}", commitSha);

      if (configuration.IncludePullRequests)
      {
        var request = new PullRequestRequest {State = ItemStateFilter.All, Base = commitSha};
        var pullRequests = await gitHubContext.GetPullRequests(request);
        context.Logger.LogInformation("Found {PullRequestCount} pull requests for commit {CommitSha}.",
          pullRequests.Count, commitSha);
      }

      if (configuration.IncludeIssues)
      {
        var request = new RepositoryIssueRequest {State = ItemStateFilter.All, SortProperty = IssueSort.Created};
        var issues = await gitHubContext.GetIssues(request);
        context.Logger.LogInformation(
          "Found {IssueCount} issues for repository {Repository}.",
          issues.Count,
          gitHubContext.Repository.FullName
        );
      }
    }

    return MiddlewareResult.Success();
  }
}
