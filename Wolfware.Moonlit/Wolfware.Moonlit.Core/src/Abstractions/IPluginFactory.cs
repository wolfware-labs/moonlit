using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Represents a factory interface for creating plugin instances.
/// </summary>
public interface IPluginFactory
{
  /// <summary>
  /// Asynchronously creates a plugin instance using the provided plugin configuration and global configuration settings.
  /// </summary>
  /// <param name="configuration">
  /// The <see cref="PluginConfiguration"/> containing the settings and metadata required to initialize the plugin.
  /// </param>
  /// <param name="releaseConfiguration">
  /// The global <see cref="IConfiguration"/> instance containing release-wide configuration settings.
  /// </param>
  /// <param name="cancellationToken">
  /// A <see cref="CancellationToken"/> to observe while waiting for the task to complete, allowing the operation to be cancelled.
  /// </param>
  /// <returns>
  /// A task representing the asynchronous operation. The task result is an instance of <see cref="IPlugin"/> representing the created plugin.
  /// </returns>
  Task<IPlugin> CreatePlugin(PluginConfiguration configuration, IConfiguration releaseConfiguration,
    CancellationToken cancellationToken = default);
}
