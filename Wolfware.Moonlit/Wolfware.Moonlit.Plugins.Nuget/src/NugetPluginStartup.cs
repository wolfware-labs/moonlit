using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Nuget.Configuration;
using Wolfware.Moonlit.Plugins.Nuget.Middlewares;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Nuget;

public class NugetPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.Configure<NugetConfiguration>(configuration);
  }

  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<PackProject>("pack");
    services.AddMiddleware<PushPackage>("push");
  }
}
