using Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Services;

public sealed class PullRequestsInformationProvider : IPullRequestsInformationProvider
{
  public Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    PullRequestsInformationFetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default
  )
  {
    throw new NotImplementedException();
  }
}
