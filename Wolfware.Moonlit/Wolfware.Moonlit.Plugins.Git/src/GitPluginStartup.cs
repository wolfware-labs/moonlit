using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Git.Middlewares;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Git;

public sealed class GitPluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<GetRepositoryContext>("repo-context");
    middlewares.Add<GetLatestTag>("latest-tag");
  }
}
