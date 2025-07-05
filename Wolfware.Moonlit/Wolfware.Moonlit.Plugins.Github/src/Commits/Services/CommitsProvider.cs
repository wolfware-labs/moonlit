using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Models;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Services;

public sealed class CommitsProvider : ICommitsProvider
{
  public Task<IReadOnlyList<GitHubCommit>> GetItems(IGitHubContext context, FetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default)
  {
    throw new NotImplementedException();
  }
}
