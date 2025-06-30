using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Defines a factory interface responsible for creating instances of <see cref="IPluginsContext"/>.
/// </summary>
public interface IPluginsContextFactory
{
  /// <summary>
  /// Creates an instance of <see cref="IPluginsContext"/> based on the provided plugin configurations and release configuration.
  /// </summary>
  /// <param name="pluginConfigurations">
  /// An array of <see cref="PluginConfiguration"/> that defines the configurations for plugins.
  /// </param>
  /// <param name="releaseConfiguration">
  /// The <see cref="IConfiguration"/> instance representing the release configuration.
  /// </param>
  /// <param name="cancellationToken">
  /// An optional <see cref="CancellationToken"/> to observe while waiting for the task to complete.
  /// </param>
  /// <returns>
  /// A task representing the asynchronous operation. The task result contains the instantiated <see cref="IPluginsContext"/>.
  /// </returns>
  Task<IPluginsContext> CreateContext(PluginConfiguration[] pluginConfigurations, IConfiguration releaseConfiguration,
    CancellationToken cancellationToken = default);
}
