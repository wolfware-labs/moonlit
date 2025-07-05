using Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Branches.Configuration;
using Wolfware.Moonlit.Plugins.Github.Branches.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Branches.Services;

public sealed class BranchesInformationProvider : IBranchesInformationProvider
{
  public Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    BranchesInformationFetchConfiguration fetchConfiguration
  )
  {
    throw new NotImplementedException();
  }
}
