using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Dotnet.Middlewares;

public sealed class BuildProject : ReleaseMiddleware<BuildProjectConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    BuildProjectConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
