using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Dotnet.Middlewares;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Dotnet;

public sealed class DotnetPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.Configure<DotnetConfiguration>(configuration);
  }

  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<BuildProject>("build");
    middlewares.Add<PackProject>("pack");
    middlewares.Add<PushPackage>("push");
    middlewares.Add<RunUnitTests>("test");
  }
}
