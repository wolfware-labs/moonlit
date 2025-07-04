using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Octokit;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
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
    var githubConfig = configuration.GetRequired<GitHubConfiguration>();
    if (string.IsNullOrWhiteSpace(githubConfig.Owner))
    {
      throw new InvalidOperationException("GitHub owner is not configured.");
    }

    if (string.IsNullOrWhiteSpace(githubConfig.Repository))
    {
      throw new InvalidOperationException("GitHub repository is not configured.");
    }

    if (string.IsNullOrWhiteSpace(githubConfig.Token))
    {
      throw new InvalidOperationException("GitHub token is not configured.");
    }

    services.Configure<GitHubConfiguration>(config =>
    {
      config.Repository = githubConfig.Repository;
      config.Owner = githubConfig.Owner;
      config.Token = githubConfig.Token;
    });

    services.AddSingleton<IGitHubClient>(svc =>
    {
      var githubConfig = svc.GetRequiredService<IOptions<GitHubConfiguration>>().Value;
      return new GitHubClient(new ProductHeaderValue("Wolfware.Moonlit.Plugins.Github"))
      {
        Credentials = new Credentials(githubConfig.Token)
      };
    });
    services.AddMiddleware<AnnotateAffectedIssues>("annotate-issues");
    services.AddMiddleware<AnnotateAffectedPullRequests>("annotate-pr");
    services.AddMiddleware<CreateRelease>("create-release");
  }
}
