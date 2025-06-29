using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Represents a factory interface for creating plugin instances.
/// </summary>
public interface IPluginFactory
{
  /// <summary>
  /// Asynchronously creates an instance of a plugin based on the provided configuration.
  /// </summary>
  /// <param name="configuration">
  /// The configuration details required to create the plugin, including name, URL, and specific settings.
  /// </param>
  /// <param name="cancellationToken">
  /// A cancellation token that can be used to cancel the operation.
  /// </param>
  /// <returns>
  /// A task that represents the asynchronous operation. The task result contains the created plugin instance.
  /// </returns>
  Task<IPlugin> CreatePlugin(PluginConfiguration configuration, CancellationToken cancellationToken = default);
}
