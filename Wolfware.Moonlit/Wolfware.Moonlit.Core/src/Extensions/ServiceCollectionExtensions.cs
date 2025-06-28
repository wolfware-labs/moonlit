using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Plugins;

namespace Wolfware.Moonlit.Core.Extensions;

public static class ServiceCollectionExtensions
{
  public static IServiceCollection AddMoonlitCore(this IServiceCollection services)
  {
    services.AddSingleton<IReleaseConfigurationParser, YamlConfigurationParser>();
    services.AddSingleton<IPluginProvider, PluginProvider>();
    return services;
  }
}
