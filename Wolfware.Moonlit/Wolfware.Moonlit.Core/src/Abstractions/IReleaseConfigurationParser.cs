using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Defines a mechanism to parse release configurations from a string and produce a <see cref="ReleaseConfiguration"/> object.
/// </summary>
public interface IReleaseConfigurationParser
{
  /// <summary>
  /// Parses a release configuration from the provided string and returns a <see cref="ReleaseConfiguration"/> object.
  /// </summary>
  /// <param name="configuration">The string containing the serialized release configuration in a predefined format.</param>
  /// <param name="cancellationToken">A token to monitor for cancellation requests, with a default value of <see cref="CancellationToken.None"/>.</param>
  /// <returns>A task representing the asynchronous operation. The task result contains the parsed <see cref="ReleaseConfiguration"/> object.</returns>
  Task<ReleaseConfiguration> Parse(string configuration, CancellationToken cancellationToken = default);
}
