using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.NodeJs.Middlewares;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.NodeJs;

public sealed class NodeJsPluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<BuildProject>("build");
    middlewares.Add<PackProject>("pack");
    middlewares.Add<PushPackage>("push");
  }
}
