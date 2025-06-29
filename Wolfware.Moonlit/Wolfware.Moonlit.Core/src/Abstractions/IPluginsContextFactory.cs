using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPluginsContextFactory
{
  Task<IPluginsContext> CreateContext(PluginConfiguration[] pluginConfigurations, CancellationToken cancellationToken = default);
}
