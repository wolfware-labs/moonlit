using Wolfware.Moonlit.Plugins.Moonlit.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Moonlit.Middlewares;

public sealed class RunModules : ReleaseMiddleware<RunModulesConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, RunModulesConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
