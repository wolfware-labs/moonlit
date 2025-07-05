using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Services;

public sealed class PullRequestsInformationProvider : IPullRequestsInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public PullRequestsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    PullRequestsInformationFetchConfiguration fetchConfiguration
  )
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);
    return new Dictionary<string, object>();
  }
}
