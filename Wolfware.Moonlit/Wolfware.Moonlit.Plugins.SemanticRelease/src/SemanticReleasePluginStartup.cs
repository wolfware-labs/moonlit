using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

namespace Wolfware.Moonlit.Plugins.SemanticRelease;

/// <summary>
/// Configures services and settings specifically for the plugin's startup process.
/// </summary>
public sealed class SemanticReleasePluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<CalculateVersion>("calculate-version");
    services.AddMiddleware<GenerateChangelog>("generate-changelog");
  }
}
