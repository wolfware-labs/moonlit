using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPluginFactory
{
  Task<IPlugin> CreatePlugin(PluginConfiguration configuration, CancellationToken cancellationToken = default);
}
