using System.Reflection;
using McMaster.NETCore.Plugins;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Configuration.Abstractions;
using Wolfware.Moonlit.Core.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

/// <summary>
/// Provides functionality for creating and initializing plugin instances.
/// </summary>
public sealed class PluginFactory : IPluginFactory
{
  private readonly IPluginPathResolver _pluginPathResolver;
  private readonly IConfigurationFactory _configurationFactory;
  private readonly IEnumerable<ILoggerProvider> _loggerProviders;

  public PluginFactory(IPluginPathResolver pluginPathResolver, IConfigurationFactory configurationFactory,
    IEnumerable<ILoggerProvider> loggerProviders)
  {
    _pluginPathResolver = pluginPathResolver;
    _configurationFactory = configurationFactory;
    _loggerProviders = loggerProviders;
  }

  public async Task<IPlugin> CreatePlugin(PluginConfiguration configuration, IConfiguration releaseConfiguration,
    CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var loader = await GetPluginLoader(configuration, cancellationToken).ConfigureAwait(false);
    var defaultPluginAssembly = loader.LoadDefaultAssembly();
    var startupInstance = PluginFactory.CreateStartupInstance(defaultPluginAssembly);
    var services = new ServiceCollection();
    var config = this._configurationFactory.Create(configuration.Configuration, releaseConfiguration);
    services.AddSingleton(config);
    services.AddLogging(cfg =>
    {
      foreach (var provider in this._loggerProviders)
      {
        cfg.Services.AddSingleton(provider);
      }
    });
    startupInstance.Configure(services, config);
    var provider = services.BuildServiceProvider();
    return new Plugin(loader, provider);
  }

  private async Task<PluginLoader> GetPluginLoader(PluginConfiguration configuration,
    CancellationToken cancellationToken = default)
  {
    var pluginPath = await this._pluginPathResolver.GetPluginPath(configuration.Url, cancellationToken)
      .ConfigureAwait(false);
    return PluginLoader.CreateFromAssemblyFile(
      pluginPath,
      sharedTypes: [typeof(IPluginStartup), typeof(IReleaseMiddleware), typeof(ILogger<>)],
      isUnloadable: true
    );
  }

  private static IPluginStartup CreateStartupInstance(Assembly pluginAssembly)
  {
    var startupType = pluginAssembly.GetTypes()
      .FirstOrDefault(t => typeof(IPluginStartup).IsAssignableFrom(t) && !t.IsAbstract);

    if (startupType == null)
    {
      throw new InvalidOperationException("No valid startup type found.");
    }

    var startupInstance = (IPluginStartup?)Activator.CreateInstance(startupType)
                          ?? throw new InvalidOperationException(
                            $"Failed to create instance of startup type '{startupType.FullName}'.");
    if (startupInstance == null)
    {
      throw new InvalidOperationException($"Startup instance of type '{startupType.FullName}' is null.");
    }

    return startupInstance;
  }
}
