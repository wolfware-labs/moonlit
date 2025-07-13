using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Middlewares;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git;

public sealed class GitPluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<GetRepositoryContext>("repo-context");
  }
}
