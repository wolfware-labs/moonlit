using McMaster.NETCore.Plugins;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class Plugin : IPlugin
{
  private readonly PluginLoader _pluginLoader;
  private readonly ServiceProvider _serviceProvider;

  public Plugin(PluginLoader pluginLoader, ServiceProvider serviceProvider)
  {
    _pluginLoader = pluginLoader;
    _serviceProvider = serviceProvider;
  }

  public IReleaseMiddleware GetMiddleware(string name)
  {
    return _serviceProvider.GetKeyedService<IReleaseMiddleware>(name)
           ?? throw new KeyNotFoundException($"Middleware with name '{name}' not found.");
  }

  public ValueTask DisposeAsync()
  {
    _pluginLoader.Dispose();
    return _serviceProvider.DisposeAsync();
  }
}
