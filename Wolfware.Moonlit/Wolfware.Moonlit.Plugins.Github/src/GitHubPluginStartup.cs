using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Octokit;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Github.Middlewares;
using Wolfware.Moonlit.Plugins.Pipelines;
using GitHubContextFactory = Wolfware.Moonlit.Plugins.Github.Services.GitHubContextFactory;

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
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
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

  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<GetLatestTag>("latest-tag");
    middlewares.Add<GetItemsSinceCommit>("items-since-commit");
    middlewares.Add<CreateRelease>("create-release");
    middlewares.Add<WriteVariables>("write-variables");
  }
}
