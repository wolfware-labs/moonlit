using Wolfware.Moonlit.Plugins.Github.Commits.Models;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;

public interface ICommitsProvider : IItemsProvider<GitHubCommit>;
