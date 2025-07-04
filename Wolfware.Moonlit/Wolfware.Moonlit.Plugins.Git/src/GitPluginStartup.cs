using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Git.Middlewares;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git;

/// <summary>
/// Provides the startup implementation for a plugin within the Wolfware.Moonlit.Plugins.Git namespace.
/// Configures required services and middlewares for plugin functionality.
/// </summary>
internal sealed class GitPluginStartup : PluginStartup
{
  public override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.Configure<GitConfiguration>(configuration);
  }

  public override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<GetGitInformation>("info");
    services.AddMiddleware<GetCommits>("commits");
    services.AddMiddleware<CreateTag>("tag");
  }
}
