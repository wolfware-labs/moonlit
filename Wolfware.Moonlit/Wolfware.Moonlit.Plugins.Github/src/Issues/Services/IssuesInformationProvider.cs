using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Issues.Services;

public sealed class IssuesInformationProvider : IIssuesInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public IssuesInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    IssuesInformationFetchConfiguration fetchConfiguration
  )
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);
    return new Dictionary<string, object>();
  }
}
