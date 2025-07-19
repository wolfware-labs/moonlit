using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

namespace Wolfware.Moonlit.Plugins.SemanticRelease;

/// <summary>
/// Configures services and settings specifically for the plugin's startup process.
/// </summary>
public sealed class SemanticReleasePluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<CalculateVersion>("calculate-version");
    middlewares.Add<GenerateChangelog>("generate-changelog");
  }
}
