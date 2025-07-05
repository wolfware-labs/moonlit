using Wolfware.Moonlit.Plugins.Github.Branches.Configuration;
using Wolfware.Moonlit.Plugins.Github.Branches.Models;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

namespace Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;

public interface IBranchesInformationProvider : IItemsInformationProvider<BranchesInformationFetchConfiguration>;
