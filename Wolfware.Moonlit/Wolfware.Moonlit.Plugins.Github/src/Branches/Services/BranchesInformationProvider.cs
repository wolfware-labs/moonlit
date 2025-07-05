using Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Branches.Configuration;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Branches.Services;

public sealed class BranchesInformationProvider : IBranchesInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public BranchesInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    BranchesInformationFetchConfiguration fetchConfiguration
  )
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context);
    return new Dictionary<string, object>();
  }
}
