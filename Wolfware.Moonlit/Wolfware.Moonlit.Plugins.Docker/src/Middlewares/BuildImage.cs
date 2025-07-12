using Wolfware.Moonlit.Plugins.Docker.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Docker.Middlewares;

public sealed class BuildImage : ReleaseMiddleware<BuildImageConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, BuildImageConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
