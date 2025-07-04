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
    services.AddSingleton<IConfigurationExpressionParser, ConfigurationExpressionParser>();

    services.AddFilePathResolver<NugetPackageResolver>("nuget");
    services.AddFilePathResolver<FilePathResolver>("file");
    services.AddFilePathResolver<HttpPathResolver>("http", "https");
    return services;
  }

  public static IServiceCollection AddFilePathResolver<T>(this IServiceCollection services, params string[] fileSchemas)
    where T : class, IFilePathResolver
  {
    ArgumentNullException.ThrowIfNull(services);
    ArgumentNullException.ThrowIfNull(fileSchemas);

    if (fileSchemas.Length == 0)
    {
      throw new ArgumentException("At least one file schema must be provided.", nameof(fileSchemas));
    }

    services.AddKeyedSingleton<IFilePathResolver, T>(fileSchemas[0]);
    for (int i = 1; i < fileSchemas.Length; i++)
    {
      services.AddKeyedSingleton<IFilePathResolver>(fileSchemas[i],
        (svc, _) => svc.GetRequiredKeyedService<IFilePathResolver>(fileSchemas[0]));
    }

    return services;
  }
}
