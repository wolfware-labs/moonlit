using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Models;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Services;

public sealed class PullRequestsProvider : IPullRequestsProvider
{
  public Task<IReadOnlyList<GitHubPullRequest>> GetItems(IGitHubContext context, FetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default)
  {
    throw new NotImplementedException();
  }
}
