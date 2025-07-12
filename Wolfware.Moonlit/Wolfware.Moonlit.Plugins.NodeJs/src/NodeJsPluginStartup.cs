using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.NodeJs.Middlewares;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.NodeJs;

public sealed class NodeJsPluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<BuildProject>("build");
    services.AddMiddleware<PackProject>("pack");
    services.AddMiddleware<PushPackage>("push");
  }
}
