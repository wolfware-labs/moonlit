using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Github.Issues.Models;
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
    var issuesContext = new IssuesFetchContext();
  }
}
