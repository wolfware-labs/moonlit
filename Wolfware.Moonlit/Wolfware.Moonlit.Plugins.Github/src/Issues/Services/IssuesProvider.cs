using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Models;

namespace Wolfware.Moonlit.Plugins.Github.Issues.Services;

public sealed class IssuesProvider : IIssuesProvider
{
  public Task<IReadOnlyList<GitHubIssue>> GetItems(IGitHubContext context, FetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default)
  {
    throw new NotImplementedException();
  }
}
