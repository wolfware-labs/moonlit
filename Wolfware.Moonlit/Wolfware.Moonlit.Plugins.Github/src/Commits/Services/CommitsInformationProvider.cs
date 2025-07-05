using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Configuration;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Services;

public sealed class CommitsInformationProvider : ICommitsInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public CommitsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    CommitsInformationFetchConfiguration fetchConfiguration
  )
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);
    return new Dictionary<string, object>();
  }
}
