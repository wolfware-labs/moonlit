using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

/// <summary>
/// Provides a sealed implementation of <see cref="IPluginsContext"/> for managing plugins and ensuring proper asynchronous disposal of resources.
/// </summary>
/// <remarks>
/// The <see cref="PluginsContext"/> class manages a collection of plugins, allowing retrieval by name and supports asynchronous cleanup of allocated resources.
/// </remarks>
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
