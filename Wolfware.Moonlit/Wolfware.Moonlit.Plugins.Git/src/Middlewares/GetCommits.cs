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

public sealed class GetCommits : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.GetRequired<FilterCommitsConfiguration>();
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();

    using var gitRepo = new Repository(gitFolderPath);

    context.Logger.LogInformation("Collecting commit history from Git repository at {GitFolderPath}", gitFolderPath);
    var tagRegex = config.GetTagRegex();
    var latestTag = gitRepo.Tags
      .Where(tag => tagRegex.IsMatch(tag.FriendlyName) && tag.Target is Commit)
      .Select(tag => new {Tag = tag, Commit = (Commit)tag.Target})
      .OrderByDescending(tag => tag.Commit?.Author.When ?? DateTimeOffset.MinValue)
      .FirstOrDefault();

    context.Logger.LogInformation("Latest tag found: {LatestTag}", latestTag?.Tag.FriendlyName ?? "None");

    CommitMessage[] commits;

    if (latestTag == null)
    {
      commits = gitRepo.Commits
        .QueryBy(new CommitFilter {SortBy = CommitSortStrategies.Topological | CommitSortStrategies.Time})
        .Select(CreateCommitMessage)
        .ToArray();
    }
    else
    {
      commits = gitRepo.Commits
        .QueryBy(new CommitFilter {IncludeReachableFrom = gitRepo.Head.Tip, ExcludeReachableFrom = latestTag.Commit})
        .Select(CreateCommitMessage)
        .ToArray();
    }

    context.Logger.LogInformation("Collected {CommitCount} commits since last tag.", commits.Length);

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("latestTag", latestTag?.Tag.FriendlyName);
      output.Add("commits", commits);
    }));
  }

  private CommitMessage CreateCommitMessage(Commit commit)
  {
    return new CommitMessage
    {
      Sha = commit.Sha,
      Author = commit.Author.Name,
      Email = commit.Author.Email,
      Date = commit.Author.When,
      Message = commit.Message.Trim()
    };
  }
}
