using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using Octokit;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Branches.Services;
using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Services;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Configuration;
using Wolfware.Moonlit.Plugins.Github.Core.Middlewares;
using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Middlewares;
using Wolfware.Moonlit.Plugins.Github.Issues.Services;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Middlewares;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Services;
using Wolfware.Moonlit.Plugins.Github.Releases.Middlewares;
using Wolfware.Moonlit.Plugins.Github.Tags.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Services;
using Wolfware.Moonlit.Plugins.Pipeline;
using GitHubContextFactory = Wolfware.Moonlit.Plugins.Github.Core.Services.GitHubContextFactory;

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
    services.AddSingleton<IBranchesInformationProvider, BranchesInformationProvider>();
    services.AddSingleton<ITagsInformationProvider, TagsInformationProvider>();
    services.AddSingleton<ICommitsInformationProvider, CommitsInformationProvider>();
    services.AddSingleton<IPullRequestsInformationProvider, PullRequestsInformationProvider>();
    services.AddSingleton<IIssuesInformationProvider, IssuesInformationProvider>();
  }

  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<GetGitInformation>("info");
    services.AddMiddleware<CreateRelease>("create-release");
    services.AddMiddleware<AnnotateAffectedIssues>("annotate-issues");
    services.AddMiddleware<AnnotateAffectedPullRequests>("annotate-pr");
  }
}
