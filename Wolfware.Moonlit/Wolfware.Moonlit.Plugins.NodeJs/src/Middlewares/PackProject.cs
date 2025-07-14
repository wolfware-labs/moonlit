using Wolfware.Moonlit.Plugins.NodeJs.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.NodeJs.Middlewares;

public sealed class PackProject : ReleaseMiddleware<PackProjectConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, PackProjectConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
