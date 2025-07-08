using System.Collections.Concurrent;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

/// <summary>
/// A factory class responsible for creating instances of <see cref="IPluginsContext"/>.
/// </summary>
/// <remarks>
/// This class utilizes an instance of <see cref="IPluginFactory"/> to create plugins based on the provided
/// plugin configurations. It ensures that at least one plugin configuration is supplied when creating a context
/// and initializes a plugins dictionary.
/// The created context encapsulates a collection of plugins mapped by their configuration names.
/// </remarks>
/// <threadsafety>
/// This class is thread-safe for concurrent use when providing non-overlapping instances of the underlying
/// dependency <see cref="IPluginFactory"/>. Shared plugin dependencies should be managed accordingly to prevent
/// race conditions or unexpected behavior.
/// </threadsafety>
public sealed class PluginsContextFactory : IPluginsContextFactory
{
  private readonly IPluginFactory _pluginFactory;

  public PluginsContextFactory(IPluginFactory pluginFactory)
  {
    _pluginFactory = pluginFactory;
  }

  /// <inheritdoc />
  public async Task<IPluginsContext> CreateContext(
    PluginConfiguration[] pluginConfigurations,
    IConfiguration releaseConfiguration,
    CancellationToken cancellationToken = default
  )
  {
    ArgumentNullException.ThrowIfNull(pluginConfigurations, nameof(pluginConfigurations));
    if (pluginConfigurations.Length == 0)
    {
      throw new ArgumentException("At least one plugin configuration must be provided.", nameof(pluginConfigurations));
    }


    var plugins = new ConcurrentDictionary<string, IPlugin>();
    await Parallel.ForEachAsync(pluginConfigurations, cancellationToken, async (pc, ct) =>
    {
      var plugin = await _pluginFactory.CreatePlugin(pc, releaseConfiguration, ct)
        .ConfigureAwait(false);
      plugins.TryAdd(pc.Name, plugin);
    });

    return new PluginsContext(plugins);
  }
}
