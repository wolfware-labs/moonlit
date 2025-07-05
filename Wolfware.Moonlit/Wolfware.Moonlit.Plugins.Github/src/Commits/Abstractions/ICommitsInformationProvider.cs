using Wolfware.Moonlit.Plugins.Github.Commits.Configuration;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;

public interface ICommitsInformationProvider : IItemsInformationProvider<CommitsInformationFetchConfiguration>;
