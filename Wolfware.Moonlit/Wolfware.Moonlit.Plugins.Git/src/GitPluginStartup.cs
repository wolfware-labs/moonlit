using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Git.Middlewares;
using Wolfware.Moonlit.Plugins.Git.Services;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Git;

public sealed class GitPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.AddSingleton<SharedContext>();
  }

  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<GetRepositoryContext>("repo-context");
    middlewares.Add<GetLatestTag>("latest-tag");
    middlewares.Add<GetCommits>("commits");
  }
}
