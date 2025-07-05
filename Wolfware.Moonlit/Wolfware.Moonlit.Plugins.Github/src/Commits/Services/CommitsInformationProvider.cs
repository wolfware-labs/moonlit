using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Services;

public sealed class CommitsInformationProvider : ICommitsInformationProvider
{
  public Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    CommitsInformationFetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default
  )
  {
    throw new NotImplementedException();
  }
}
