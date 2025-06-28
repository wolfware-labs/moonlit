using McMaster.NETCore.Plugins;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class Plugin : IPlugin
{
  private readonly IServiceProvider _serviceProvider;

  public Plugin(IServiceProvider serviceProvider)
  {
    _serviceProvider = serviceProvider;
  }

  public static PluginLoader Load(string assemblyPath, IServiceCollection services, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(assemblyPath, nameof(assemblyPath));
    ArgumentNullException.ThrowIfNull(services, nameof(services));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var loader = PluginLoader.CreateFromAssemblyFile(
      assemblyPath,
      sharedTypes: [typeof(IPluginStartup), typeof(IPipelineMiddleware)],
      isUnloadable: true
    );
    var defaultPluginAssembly = loader.LoadDefaultAssembly();
    var startupType = defaultPluginAssembly.GetTypes()
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

    startupInstance.Configure(services, configuration);
    return loader;
  }

  public IPipelineMiddleware GetMiddleware(string name)
  {
    return _serviceProvider.GetKeyedService<IPipelineMiddleware>(name)
           ?? throw new KeyNotFoundException($"Middleware with name '{name}' not found.");
  }
}
