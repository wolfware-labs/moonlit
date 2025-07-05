using Octokit;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Services;

public sealed class PullRequestsInformationProvider : IPullRequestsInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public PullRequestsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task PopulateFetchContext(
    ReleaseContext releaseContext,
    PullRequestsInformationFetchConfiguration fetchConfiguration,
    FetchContext fetchContext
  )
  {
    if (fetchConfiguration.Strategy == PullRequestsFetchStrategy.None)
    {
      return;
    }

    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(releaseContext);
    fetchContext.PullRequests = new PullRequestsFetchContext();
    if (fetchConfiguration.Strategy == PullRequestsFetchStrategy.FromAvailableCommits)
    {
      if (fetchContext.Commits is not {Commits.Length: > 0})
      {
        throw new InvalidOperationException(
          "Pull requests fetch strategy 'FromAvailableCommits' requires commits to be available in the fetch context."
        );
      }

      var prs = await gitHubContext.GetPullRequests(new PullRequestRequest {State = ItemStateFilter.All});
      if (prs.Count == 0)
      {
        return;
      }

      fetchContext.PullRequests.PullRequests = prs
        .Where(pr => pr.MergeCommitSha != null && fetchContext.Commits.Commits.Any(c => c.Sha == pr.MergeCommitSha))
        .Select(pr => new PullRequestInformation
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
  }
}
