using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Git.Models;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

internal sealed class GetGitInformation : ReleaseMiddleware<GetGitInformation.Configuration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public sealed class Configuration
  {
    public bool CollectBranches { get; set; }

    public bool CollectTags { get; set; }

    public bool CollectCommits { get; set; }
  }

  public GetGitInformation(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, Configuration configuration)
  {
    context.Logger.LogInformation("Collecting Git information from repository");

    var githubContext = await _gitHubContextProvider.GetCurrentContext(context);

    IReadOnlyList<Branch> branches = [];
    if (configuration.CollectBranches)
    {
      context.Logger.LogInformation("Collecting branches...");
      branches = await githubContext.GetBranches(new ApiOptions());
      context.Logger.LogInformation("Found {BranchCount} branches.", branches.Count);
    }

    IReadOnlyList<RepositoryTag> tags = [];
    if (configuration.CollectTags)
    {
      context.Logger.LogInformation("Collecting tags...");
      tags = await githubContext.GetTags(new ApiOptions());
      context.Logger.LogInformation("Found {TagCount} tags.", tags.Count);
    }

    IReadOnlyList<GitHubCommit> commits = [];
    if (configuration.CollectCommits)
    {
      context.Logger.LogInformation("Collecting commits...");
      commits = await githubContext.GetCommits(new CommitRequest {Sha = githubContext.CurrentBranch,});
      context.Logger.LogInformation("Found {CommitCount} commits.", commits.Count);
    }

    context.Logger.LogInformation("Git information collection completed.");
    return MiddlewareResult.Success(output =>
    {
      output.Add("Branch", githubContext.CurrentBranch);

      if (branches is {Count: > 0})
      {
        output.Add("Branches", branches);
      }

      if (tags is {Count: > 0})
      {
        output.Add("Tags", tags);
      }

      if (commits is {Count: > 0})
      {
        output.Add("Commits", commits);
      }
    });
  }
}
