using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginProvider : IPluginProvider
{
  private readonly IServiceProvider _serviceProvider;

  public PluginProvider(IServiceProvider serviceProvider)
  {
    _serviceProvider = serviceProvider;
  }

  public IPlugin GetPlugin(string name)
  {
    ArgumentNullException.ThrowIfNull(name, nameof(name));
    var plugin = _serviceProvider.GetKeyedService<IPlugin>(name);
    if (plugin is null)
    {
      throw new KeyNotFoundException($"Plugin with name '{name}' not found.");
    }

    return plugin;
  }
}
