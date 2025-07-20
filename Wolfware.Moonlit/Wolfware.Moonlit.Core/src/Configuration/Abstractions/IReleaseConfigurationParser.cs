namespace Wolfware.Moonlit.Core.Configuration.Abstractions;

/// <summary>
/// Defines a mechanism to parse release configurations from a string and produce a <see cref="ReleaseConfiguration"/> object.
/// </summary>
public interface IReleaseConfigurationParser
{
  /// <summary>
  /// Parses the provided configuration string and returns a <see cref="ReleaseConfiguration"/> object
  /// representing the release process settings.
  /// </summary>
  /// <param name="configuration">The configuration string to parse into a <see cref="ReleaseConfiguration"/> object.</param>
  /// <returns>A <see cref="ReleaseConfiguration"/> object containing the parsed release process settings.</returns>
  ReleaseConfiguration Parse(string configuration);
}
