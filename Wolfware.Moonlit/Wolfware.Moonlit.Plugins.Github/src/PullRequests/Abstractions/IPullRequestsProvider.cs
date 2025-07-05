using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Models;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;

public interface IPullRequestsProvider : IItemsProvider<GitHubPullRequest>
{
}
