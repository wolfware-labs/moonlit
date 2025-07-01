using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Middlewares;

namespace Wolfware.Moonlit.Plugins.Git;

/// <summary>
/// Provides the startup implementation for a plugin within the Wolfware.Moonlit.Plugins.Git namespace.
/// Configures required services and middlewares for plugin functionality.
/// </summary>
public sealed class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    services.AddMiddleware<CollectCommitHistory>("collect-commit-history");
    services.AddMiddleware<CreateTag>("tag");
  }
}
