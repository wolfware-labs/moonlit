using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Github.Middlewares;

namespace Wolfware.Moonlit.Plugins.Github;

/// <summary>
/// Responsible for configuring services and middleware required by the GitHub plugin.
/// </summary>
/// <remarks>
/// This implementation of <see cref="IPluginStartup"/> registers specific middleware components
/// to the service collection, enabling GitHub-related functionalities like issue annotations,
/// pull request annotations, and release creation.
/// </remarks>
public sealed class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    services.AddMiddleware<AnnotateAffectedIssues>("annotate-issues");
    services.AddMiddleware<AnnotateAffectedPullRequests>("annotate-pr");
    services.AddMiddleware<CreateRelease>("create-release");
  }
}
