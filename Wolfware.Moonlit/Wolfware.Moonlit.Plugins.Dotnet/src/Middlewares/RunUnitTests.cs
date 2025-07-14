using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Dotnet.Middlewares;

public sealed class RunUnitTests : ReleaseMiddleware<RunUnitTestsConfiguration>
{
  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, RunUnitTestsConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
