using LibGit2Sharp;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Git.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

internal sealed class GetGitInformation : ReleaseMiddleware<GetGitInformation.Configuration>
{
  public sealed class Configuration
  {
    public bool CollectBranches { get; set; }

    public bool CollectTags { get; set; }

    public bool CollectCommits { get; set; }
  }

  public override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, Configuration configuration)
  {
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();

    using var gitRepo = new Repository(gitFolderPath);

    context.Logger.LogInformation("Collecting Git information from repository at {GitFolderPath}", gitFolderPath);

    string currentBranch = gitRepo.Head.FriendlyName;

    string[]? branches = null;
    if (configuration.CollectBranches)
    {
      context.Logger.LogInformation("Collecting branches...");
      branches = gitRepo.Branches.Select(b => b.FriendlyName).ToArray();
      context.Logger.LogInformation("Found {BranchCount} branches.", branches.Length);
    }

    string[]? tags = null;
    if (configuration.CollectTags)
    {
      context.Logger.LogInformation("Collecting tags...");
      tags = gitRepo.Tags.Select(t => t.FriendlyName).ToArray();
      context.Logger.LogInformation("Found {TagCount} tags.", tags.Length);
    }

    CommitMessage[]? commits = null;
    if (configuration.CollectCommits)
    {
      context.Logger.LogInformation("Collecting commits...");
      commits = gitRepo.Commits.OrderByDescending(x => x.Author.When).Select(c => new CommitMessage
      {
        Sha = c.Sha,
        Author = c.Author.Name,
        Email = c.Author.Email,
        Date = c.Author.When,
        Message = c.Message
      }).ToArray();
      context.Logger.LogInformation("Found {CommitCount} commits.", commits.Length);
    }

    context.Logger.LogInformation("Git information collection completed.");
    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("Branch", currentBranch);

      if (branches != null)
      {
        output.Add("Branches", branches);
      }

      if (tags != null)
      {
        output.Add("Tags", tags);
      }

      if (commits != null)
      {
        output.Add("Commits", commits);
      }
    }));
  }
}
