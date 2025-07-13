using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Dotnet.Middlewares;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Dotnet;

public sealed class DotnetPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.Configure<DotnetConfiguration>(configuration);
  }

  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<BuildProject>("build");
    services.AddMiddleware<PackProject>("pack");
    services.AddMiddleware<PushPackage>("push");
    services.AddMiddleware<RunUnitTests>("test");
  }
}
