using Octokit;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Github.Issues.Models;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Issues.Services;

public sealed class IssuesInformationProvider : IIssuesInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public IssuesInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task PopulateFetchContext(
    ReleaseContext releaseContext,
    IssuesInformationFetchConfiguration fetchConfiguration,
    FetchContext fetchContext
  )
  {
    if (fetchConfiguration.Strategy == IssuesFetchStrategy.None)
    {
      return;
    }

    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(releaseContext);
    fetchContext.Issues = new IssuesFetchContext();
    if (fetchConfiguration.Strategy == IssuesFetchStrategy.FromAvailablePullRequests)
    {
      if (fetchContext.PullRequests is not {PullRequests.Length: > 0})
      {
        throw new InvalidOperationException(
          "Issues fetch strategy 'FromAvailablePullRequests' requires pull requests to be available in the fetch context."
        );
      }

      var issues = await gitHubContext.GetIssues(new RepositoryIssueRequest {State = ItemStateFilter.All});
      if (issues.Count == 0)
      {
        return;
      }

      fetchContext.Issues.Issues = issues
        .Where(issue => issue.PullRequest != null &&
                        fetchContext.PullRequests.PullRequests.Any(pr => pr.Number == issue.PullRequest.Number))
        .Select(issue => new IssueInformation
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
}
