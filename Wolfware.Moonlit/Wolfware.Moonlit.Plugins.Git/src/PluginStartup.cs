using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Middlewares;

namespace Wolfware.Moonlit.Plugins.Git;

public sealed class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    services.AddMiddleware<CollectCommitHistory>("collect-commit-history");
    services.AddMiddleware<Tag>("tag");
  }
}
