using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

public class PluginsContext : IAsyncDisposable
{
  private readonly PluginRegistry _pluginRegistry;
  private readonly PluginProvider _pluginProvider;

  private PluginsContext(PluginRegistry pluginRegistry, PluginProvider pluginProvider)
  {
    this._pluginRegistry = pluginRegistry;
    this._pluginProvider = pluginProvider;
    this.PluginProvider = pluginProvider;
  }

  public IPluginProvider PluginProvider { get; }

  public static async Task<PluginsContext> CreateNew(PluginConfiguration[] plugins,
    CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(plugins, nameof(plugins));
    if (plugins.Length == 0)
    {
      throw new ArgumentException("At least one plugin configuration must be provided.", nameof(plugins));
    }


    var registry = new PluginRegistry();
    var serviceProviders = new Dictionary<string, IServiceProvider>();

    foreach (var pluginConfiguration in plugins)
    {
      var services = new ServiceCollection();
      await registry.RegisterPlugin(services, pluginConfiguration, cancellationToken);
      serviceProviders[pluginConfiguration.Name] = services.BuildServiceProvider();
    }

    var pluginProvider = new PluginProvider(serviceProviders);
    return new PluginsContext(registry, pluginProvider);
  }

  public async ValueTask DisposeAsync()
  {
    this._pluginRegistry.Dispose();
    await this._pluginProvider.DisposeAsync();
  }
}
