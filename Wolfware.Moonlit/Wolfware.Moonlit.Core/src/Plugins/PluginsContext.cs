using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

public class PluginsContext : IAsyncDisposable
{
  private readonly PluginRegistry _pluginRegistry;
  private readonly ServiceProvider _serviceProvider;

  private PluginsContext(PluginRegistry pluginRegistry, ServiceProvider serviceProvider)
  {
    _pluginRegistry = pluginRegistry;
    _serviceProvider = serviceProvider;
  }

  public IPluginProvider PluginProvider => _serviceProvider.GetRequiredService<IPluginProvider>();

  public static async Task<PluginsContext> CreateNew(PluginConfiguration[] plugins,
    CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(plugins, nameof(plugins));
    if (plugins.Length == 0)
    {
      throw new ArgumentException("At least one plugin configuration must be provided.", nameof(plugins));
    }

    var services = new ServiceCollection();
    var registry = new PluginRegistry(services);

    foreach (var pluginConfiguration in plugins)
    {
      await registry.RegisterPlugin(pluginConfiguration, cancellationToken);
    }

    var serviceProvider = services.BuildServiceProvider();
    return new PluginsContext(registry, serviceProvider);
  }

  public async ValueTask DisposeAsync()
  {
    _pluginRegistry.Dispose();
    await _serviceProvider.DisposeAsync();
  }
}
