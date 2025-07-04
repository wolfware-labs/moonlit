using System.Text;
using System.Text.RegularExpressions;
using LibGit2Sharp;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Git.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

internal sealed class GetCommits : ReleaseMiddleware<GetCommits.Configuration>
{
  public sealed class Configuration
  {
    public string? TagRegex { get; set; }

    public string? TagPrefix { get; set; }

    public string? TagSuffix { get; set; }

    public Regex GetTagRegex()
    {
      if (this.TagRegex != null)
      {
        return new Regex(this.TagRegex, RegexOptions.Compiled | RegexOptions.IgnoreCase);
      }

      var regexStringBuilder = new StringBuilder();
      if (!string.IsNullOrEmpty(this.TagPrefix))
      {
        regexStringBuilder.Append(Regex.Escape(this.TagPrefix));
      }

      regexStringBuilder.Append("[0-9]+.[0-9]+.[0-9]+");

      if (!string.IsNullOrEmpty(this.TagSuffix))
      {
        regexStringBuilder.Append(Regex.Escape(this.TagSuffix));
      }

      return new Regex(regexStringBuilder.ToString(), RegexOptions.Compiled | RegexOptions.IgnoreCase);
    }
  }

  public override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, Configuration configuration)
  {
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();

    using var gitRepo = new Repository(gitFolderPath);

    context.Logger.LogInformation("Collecting commit history from Git repository at {GitFolderPath}", gitFolderPath);
    var tagRegex = configuration.GetTagRegex();
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
