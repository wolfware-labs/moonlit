namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Represents a context for managing and interacting with plugins.
/// </summary>
/// <remarks>
/// The <see cref="IPluginsContext"/> interface allows retrieval of plugins by name
/// and ensures proper release of plugin resources through asynchronous disposal mechanisms.
/// </remarks>
public interface IPluginsContext : IAsyncDisposable
{
  /// <summary>
  /// Retrieves a plugin by its name from the context.
  /// </summary>
  /// <param name="name">The name of the plugin to retrieve.</param>
  /// <returns>An instance of <see cref="IPlugin"/> corresponding to the specified name.</returns>
  /// <exception cref="ArgumentNullException">Thrown when the provided name is null or whitespace.</exception>
  /// <exception cref="KeyNotFoundException">Thrown when a plugin with the specified name is not found in the context.</exception>
  IPlugin GetPlugin(string name);
}
