namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Provides an abstraction for resolving file paths from a given URI.
/// </summary>
public interface IFilePathResolver
{
  /// <summary>
  /// Resolves a file path from the specified assembly URI.
  /// </summary>
  /// <param name="assemblyUri">The URI of the assembly. Must be a valid URI that can be resolved to a local or temporary file path.</param>
  /// <param name="cancellationToken">A token to monitor for cancellation requests.</param>
  /// <returns>A task representing the asynchronous operation, which contains the resolved file path as a string.</returns>
  /// <exception cref="ArgumentException">Thrown when the local path of the URI is null or empty.</exception>
  /// <exception cref="FileNotFoundException">Thrown when the file does not exist at the specified path.</exception>
  /// <exception cref="NotSupportedException">Thrown when the URI scheme is not supported.</exception>
  ValueTask<string> ResolvePath(Uri assemblyUri, CancellationToken cancellationToken = default);
}
