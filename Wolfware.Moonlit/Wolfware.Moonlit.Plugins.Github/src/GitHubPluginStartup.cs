using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Octokit;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Github.Middlewares;
using Wolfware.Moonlit.Plugins.Github.Services;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github;

/// <summary>
/// Responsible for configuring services and middleware required by the GitHub plugin.
/// </summary>
/// <remarks>
/// This implementation of <see cref="IPluginStartup"/> registers specific middleware components
/// to the service collection, enabling GitHub-related functionalities like issue annotations,
/// pull request annotations, and release creation.
/// </remarks>
public sealed class GitHubPluginStartup : PluginStartup
{
  public override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.Configure<GitHubConfiguration>(configuration);
    services.AddSingleton<IGitHubClient>(svc =>
    {
      var githubConfig = svc.GetRequiredService<IOptions<GitHubConfiguration>>().Value;
      if (string.IsNullOrWhiteSpace(githubConfig.Token))
      {
        throw new InvalidOperationException("GitHub token is not configured.");
      }

      return new GitHubClient(new ProductHeaderValue("Wolfware.Moonlit.Plugins.Github"))
      {
        Credentials = new Credentials(githubConfig.Token)
      };
    });
    services.AddSingleton<IGitHubContextProvider, GitHubContextFactory>();
  }

  public override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<GetGitInformation>("info");
    services.AddMiddleware<GetCommits>("commits");
    services.AddMiddleware<AnnotateAffectedIssues>("annotate-issues");
    services.AddMiddleware<AnnotateAffectedPullRequests>("annotate-pr");
    services.AddMiddleware<CreateRelease>("create-release");
    services.AddMiddleware<CollectAssociatedItems>("collect-associated-items");
  }
}
