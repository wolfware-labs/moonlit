using Wolfware.Moonlit.Plugins.NodeJs.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.NodeJs.Middlewares;

public sealed class BuildProject : ReleaseMiddleware<BuildProjectConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    BuildProjectConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
