using LibGit2Sharp;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class CollectCommitHistory : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.GetRequired<CollectCommitHistoryConfiguration>();
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();

    using var gitRepo = new Repository(gitFolderPath);

    var tagRegex = config.GetTagRegex();
    var latestTag = gitRepo.Tags
      .Where(tag => tagRegex.IsMatch(tag.FriendlyName) && tag.Target is Commit)
      .Select(tag => new {Tag = tag, Commit = (Commit)tag.Target})
      .OrderByDescending(tag => tag.Commit?.Author.When ?? DateTimeOffset.MinValue)
      .FirstOrDefault();

    string[] commits;

    if (latestTag == null)
    {
      commits = gitRepo.Commits
        .QueryBy(new CommitFilter {SortBy = CommitSortStrategies.Topological | CommitSortStrategies.Time})
        .Select(commit => commit.Message.Trim())
        .ToArray();
    }
    else
    {
      commits = gitRepo.Commits
        .QueryBy(new CommitFilter {IncludeReachableFrom = gitRepo.Head.Tip, ExcludeReachableFrom = latestTag.Commit})
        .Select(commit => commit.Message.Trim())
        .ToArray();
    }


    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("latestTag", latestTag?.Tag.FriendlyName);
      output.Add("commits", commits);
    }));
  }
}
