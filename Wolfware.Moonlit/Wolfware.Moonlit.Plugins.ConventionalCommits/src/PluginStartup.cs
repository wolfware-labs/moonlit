using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.ConventionalCommits.Middlewares;
using Wolfware.Moonlit.Plugins.Extensions;

namespace Wolfware.Moonlit.Plugins.ConventionalCommits;

/// <summary>
/// Configures services and settings specifically for the plugin's startup process.
/// </summary>
public class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    services.AddMiddleware<CalculateVersion>("calculate-version");
  }
}
