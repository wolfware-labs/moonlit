using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Pipelines;
using Wolfware.Moonlit.Core.Plugins;
using Wolfware.Moonlit.Core.Plugins.Resolvers;

namespace Wolfware.Moonlit.Core.Extensions;

public static class ServiceCollectionExtensions
{
  public static IServiceCollection AddMoonlitCore(this IServiceCollection services)
  {
    services.AddSingleton<IReleaseConfigurationParser, YamlConfigurationParser>();
    services.AddSingleton<IConfigurationFactory, ConfigurationFactory>();
    services.AddSingleton<IPluginsContextFactory, PluginsContextFactory>();
    services.AddSingleton<IPluginFactory, PluginFactory>();
    services.AddSingleton<IReleasePipelineFactory, ReleasePipelineFactory>();
    services.AddSingleton<IPluginPathResolver, PluginPathResolver>();
    services.AddKeyedSingleton<IFilePathResolver, FilePathResolver>("file");
    services.AddKeyedSingleton<IFilePathResolver, HttpPathResolver>("http");
    services.AddKeyedSingleton<IFilePathResolver>("https",
      (svc, _) => svc.GetRequiredKeyedService<IFilePathResolver>("http"));
    return services;
  }
}
