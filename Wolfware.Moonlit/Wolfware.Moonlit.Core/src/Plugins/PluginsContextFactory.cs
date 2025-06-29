using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginsContextFactory : IPluginsContextFactory
{
  private readonly IPluginFactory _pluginFactory;

  public PluginsContextFactory(IPluginFactory pluginFactory)
  {
    _pluginFactory = pluginFactory;
  }

  public async Task<IPluginsContext> CreateContext(PluginConfiguration[] pluginConfigurations,
    CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(pluginConfigurations, nameof(pluginConfigurations));
    if (pluginConfigurations.Length == 0)
    {
      throw new ArgumentException("At least one plugin configuration must be provided.", nameof(pluginConfigurations));
    }


    var plugins = new Dictionary<string, IPlugin>();
    foreach (var pluginConfiguration in pluginConfigurations)
    {
      var plugin = await _pluginFactory.CreatePlugin(pluginConfiguration, cancellationToken).ConfigureAwait(false);
      plugins.Add(pluginConfiguration.Name, plugin);
    }

    return new PluginsContext(plugins);
  }
}
