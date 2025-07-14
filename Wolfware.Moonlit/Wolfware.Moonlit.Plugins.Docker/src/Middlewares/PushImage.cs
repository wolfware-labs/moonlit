using Wolfware.Moonlit.Plugins.Docker.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Docker.Middlewares;

public sealed class PushImage : ReleaseMiddleware<PushImageConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, PushImageConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
