using System.Text.RegularExpressions;
using LibGit2Sharp;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class CollectCommitHistory : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.Get<CollectCommitHistoryConfiguration>();
    if (config == null)
    {
      throw new InvalidOperationException("Configuration for CollectCommitHistory is not set.");
    }

    var gitFolderPath = this.GetGitFolderPath(context.WorkingDirectory);
    if (string.IsNullOrEmpty(gitFolderPath))
    {
      context.Logger.LogError("No Git repository found in the working directory: {WorkingDirectory}",
        context.WorkingDirectory);
      return Task.FromResult(MiddlewareResult.Failure("No Git repository found"));
    }

    var tagRegex = new Regex(config.TagRegex, RegexOptions.Compiled | RegexOptions.IgnoreCase);
    using var gitRepo = new Repository(gitFolderPath);
    var commits = gitRepo.Commits
      .QueryBy(new CommitFilter {SortBy = CommitSortStrategies.Topological | CommitSortStrategies.Time})
      .ToList(); // Force loading commits to ensure the repository is valid

    return Task.FromResult(MiddlewareResult.Success());
    // context.Logger.LogInformation("Collecting commit history from repository at {RepoPath}", repoPath);
    //
    // var commitMessages = new List<string>();
    //
    // try
    // {
    //   using var repo = new Repository(repoPath);
    //
    //   // Get all tags that match the regex
    //   var matchingTags = repo.Tags
    //     .Where(tag => tagRegex.IsMatch(tag.FriendlyName))
    //     .ToList();
    //
    //   if (!matchingTags.Any())
    //   {
    //     context.Logger.LogWarning("No tags matching pattern '{TagPattern}' were found", config.TagRegex);
    //     return Task.FromResult(MiddlewareResult.Continue());
    //   }
    //
    //   // Sort tags to find the most recent one (if multiple match)
    //   var targetTag = matchingTags
    //     .OrderByDescending(tag => (tag.Target as Commit)?.Author.When ?? DateTimeOffset.MinValue)
    //     .First();
    //
    //   context.Logger.LogInformation("Found matching tag: {TagName}", targetTag.FriendlyName);
    //
    //   var tagCommit = targetTag.Target as Commit;
    //   if (tagCommit == null)
    //   {
    //     context.Logger.LogWarning("Tag {TagName} does not point to a commit", targetTag.FriendlyName);
    //     return Task.FromResult(MiddlewareResult.Continue());
    //   }
    //
    //   // Walk the commit history from HEAD to the tag
    //   var filter = new CommitFilter {IncludeReachableFrom = repo.Head.Tip, ExcludeReachableFrom = tagCommit};
    //
    //   foreach (var commit in repo.Commits.QueryBy(filter))
    //   {
    //     if (!string.IsNullOrWhiteSpace(commit.Message))
    //     {
    //       commitMessages.Add(commit.Message.Trim());
    //     }
    //   }
    //
    //   context.Logger.LogInformation("Collected {Count} commit messages since tag {TagName}",
    //     commitMessages.Count, targetTag.FriendlyName);
    //
    //   // Store the results in the context data for downstream middleware
    //   return Task.FromResult(MiddlewareResult.Continue(new Dictionary<string, object>
    //   {
    //     ["CommitMessages"] = commitMessages, ["MatchingTag"] = targetTag.FriendlyName
    //   }));
    // }
    // catch (RepositoryNotFoundException)
    // {
    //   context.Logger.LogError("No Git repository found at {RepoPath}", repoPath);
    //   return Task.FromResult(MiddlewareResult.Failed("No Git repository found"));
    // }
    // catch (Exception ex)
    // {
    //   context.Logger.LogError(ex, "Error while collecting commit history");
    //   return Task.FromResult(MiddlewareResult.Failed(ex.Message));
    // }
  }

  private string? GetGitFolderPath(string path)
  {
    while (true)
    {
      if (Directory.Exists(Path.Combine(path, ".git")))
      {
        return Path.Combine(path, ".git");
      }

      var parentDirectory = Directory.GetParent(path);
      if (parentDirectory == null)
      {
        return null;
      }

      path = parentDirectory.FullName;
    }
  }
}
