namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Defines a method for resolving the file path of a plugin based on its URI.
/// </summary>
public interface IPluginPathResolver
{
  /// <summary>
  /// Resolves the file path of a plugin based on its URI.
  /// </summary>
  /// <param name="pluginUrl">The URI of the plugin to resolve the file path for.</param>
  /// <param name="cancellationToken">A cancellation token to observe while waiting for the resolution operation to complete.</param>
  /// <returns>A task that represents the asynchronous operation. The task result contains the resolved file path of the plugin.</returns>
  ValueTask<string> GetPluginPath(Uri pluginUrl, CancellationToken cancellationToken);
}
