using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Plugins;
using Wolfware.Moonlit.Core.Plugins.Resolvers;

namespace Wolfware.Moonlit.Core.Extensions;

public static class ServiceCollectionExtensions
{
  public static IServiceCollection AddMoonlitCore(this IServiceCollection services)
  {
    services.AddSingleton<IReleaseConfigurationParser, YamlConfigurationParser>();
    services.AddSingleton<IPluginProvider, PluginProvider>();
    services.AddSingleton<IConfigurationFactory, ConfigurationFactory>();
    services.AddKeyedSingleton<IAssemblyPathResolver, FilePathResolver>("file");
    services.AddKeyedSingleton<IAssemblyPathResolver, HttpPathResolver>("http");
    services.AddKeyedSingleton<IAssemblyPathResolver>("https",
      (svc, _) => svc.GetRequiredKeyedService<IAssemblyPathResolver>("http"));
    return services;
  }
}
