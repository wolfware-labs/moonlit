using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;
using Wolfware.Moonlit.Plugins.SemanticRelease.Services;

namespace Wolfware.Moonlit.Plugins.SemanticRelease;

/// <summary>
/// Configures services and settings specifically for the plugin's startup process.
/// </summary>
public sealed class SemanticReleasePluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.AddSingleton<SharedContext>();
  }

  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<ConvertCommits>("analyze");
    middlewares.Add<CalculateVersion>("calculate-version");
    middlewares.Add<GenerateChangelog>("generate-changelog");
  }
}
