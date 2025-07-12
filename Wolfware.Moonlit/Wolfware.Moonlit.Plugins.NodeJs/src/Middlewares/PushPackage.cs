using Wolfware.Moonlit.Plugins.NodeJs.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.NodeJs.Middlewares;

public sealed class PushPackage : ReleaseMiddleware<PushPackageConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, PushPackageConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
