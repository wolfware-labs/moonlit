using Wolfware.Moonlit.Plugins.Github.Commits.Models;
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
    var pullRequestsContext = new PullRequestsFetchContext();
  }
}
