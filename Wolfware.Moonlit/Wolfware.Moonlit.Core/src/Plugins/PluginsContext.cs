using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginsContext : IPluginsContext
{
  private readonly IReadOnlyDictionary<string, IPlugin> _plugins;

  public PluginsContext(IReadOnlyDictionary<string, IPlugin> plugins)
  {
    _plugins = plugins;
  }

  public IPlugin GetPlugin(string name)
  {
    if (string.IsNullOrWhiteSpace(name))
    {
      throw new ArgumentException("Plugin name cannot be null or empty.", nameof(name));
    }

    if (!_plugins.TryGetValue(name, out var plugin))
    {
      throw new KeyNotFoundException($"Plugin '{name}' not found.");
    }

    return plugin;
  }

  public async ValueTask DisposeAsync()
  {
    foreach (var plugin in _plugins.Values)
    {
      await plugin.DisposeAsync();
    }
  }
}
