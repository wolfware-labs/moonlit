using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IReleaseConfigurationParser
{
  Task<ReleaseConfiguration> Parse(string configuration,
    CancellationToken cancellationToken = default);
}
