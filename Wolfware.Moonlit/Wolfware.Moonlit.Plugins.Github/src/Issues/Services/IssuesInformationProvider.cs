using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Issues.Services;

public sealed class IssuesInformationProvider : IIssuesInformationProvider
{
  public Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    IssuesInformationFetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default
  )
  {
    throw new NotImplementedException();
  }
}
