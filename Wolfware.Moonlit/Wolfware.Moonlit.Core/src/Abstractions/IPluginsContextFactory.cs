using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Defines a factory interface responsible for creating instances of <see cref="IPluginsContext"/>.
/// </summary>
public interface IPluginsContextFactory
{
  /// <summary>
  /// Creates an instance of <see cref="IPluginsContext"/> using the provided plugin configurations.
  /// </summary>
  /// <param name="pluginConfigurations">
  /// An array of <see cref="PluginConfiguration"/> objects representing the configurations for the plugins to be initialized.
  /// </param>
  /// <param name="cancellationToken">
  /// A <see cref="CancellationToken"/> to observe while waiting for the operation to complete, default is optional.
  /// </param>
  /// <returns>
  /// A task that represents the asynchronous operation. The task result contains the created <see cref="IPluginsContext"/> instance.
  /// </returns>
  Task<IPluginsContext> CreateContext(PluginConfiguration[] pluginConfigurations,
    CancellationToken cancellationToken = default);
}
