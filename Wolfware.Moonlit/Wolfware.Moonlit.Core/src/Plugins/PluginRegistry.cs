using McMaster.NETCore.Plugins;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginRegistry : IDisposable
{
  private readonly IConfigurationFactory _configurationFactory;
  private readonly IServiceProvider _serviceProvider;
  private readonly List<PluginLoader> _pluginLoaders = [];

  public PluginRegistry(IConfigurationFactory configurationFactory, IServiceProvider serviceProvider)
  {
    _configurationFactory = configurationFactory;
    _serviceProvider = serviceProvider;
  }

  public async Task RegisterPlugin(IServiceCollection services, PluginConfiguration configuration,
    CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(configuration);
    var assemblyPath = await GetAssemblyPath(configuration.Url, cancellationToken);
    var pluginConfiguration = this._configurationFactory.Create(configuration.Configuration);
    this._pluginLoaders.Add(Plugin.Load(assemblyPath, services, pluginConfiguration));
    services.AddKeyedSingleton<IPlugin, Plugin>(configuration.Name);
  }

  private ValueTask<string> GetAssemblyPath(Uri pluginUrl, CancellationToken cancellationToken)
  {
    var resolver = this._serviceProvider.GetRequiredKeyedService<IAssemblyPathResolver>(pluginUrl.Scheme);
    if (resolver == null)
    {
      throw new NotSupportedException($"Unsupported URL scheme: {pluginUrl.Scheme}");
    }

    return resolver.ResolvePath(pluginUrl, cancellationToken);
  }

  public void Dispose()
  {
    this._pluginLoaders.ForEach(loader =>
    {
      try
      {
        loader.Dispose();
      }
      catch (Exception ex)
      {
        // Log the exception if necessary
        Console.WriteLine($"Error disposing plugin loader: {ex.Message}");
      }
    });
  }
}
