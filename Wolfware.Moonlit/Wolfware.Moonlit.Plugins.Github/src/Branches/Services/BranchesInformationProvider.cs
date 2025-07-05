using Octokit;
using Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Branches.Configuration;
using Wolfware.Moonlit.Plugins.Github.Branches.Models;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Branches.Services;

public sealed class BranchesInformationProvider : IBranchesInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public BranchesInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task PopulateFetchContext(
    ReleaseContext releaseContext,
    BranchesInformationFetchConfiguration fetchConfiguration,
    FetchContext fetchContext
  )
  {
    BranchesFetchContext? branchesContext = null;
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(releaseContext);

    if (fetchConfiguration.IncludeCurrentBranch)
    {
      branchesContext = new BranchesFetchContext {CurrentBranch = gitHubContext.CurrentBranch};
    }

    if (fetchConfiguration.IncludeRemoteBranches)
    {
      branchesContext ??= new BranchesFetchContext();
      var remoteBranches = await gitHubContext.GetBranches(new ApiOptions());
      branchesContext.Branches = remoteBranches
        .Select(branch => branch.Name)
        .ToArray();
    }

    fetchContext.Branches = branchesContext;
  }
}
