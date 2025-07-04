using System.Text.RegularExpressions;
using Octokit;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;
using GitRepository = LibGit2Sharp.Repository;

namespace Wolfware.Moonlit.Plugins.Github.Services;

public sealed partial class GitHubContextFactory : IGitHubContextProvider
{
  private readonly IGitHubClient _gitHubClient;
  private readonly SemaphoreSlim _lock = new(1, 1);
  private readonly IGitHubContext _currentContext = null!;

  public GitHubContextFactory(IGitHubClient gitHubClient)
  {
    _gitHubClient = gitHubClient;
  }

  public ValueTask<IGitHubContext> GetCurrentContext(ReleaseContext context)
  {
    if (_currentContext != null)
    {
      return new ValueTask<IGitHubContext>(_currentContext);
    }

    _lock.Wait(-1);
    try
    {
      var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();
      return _currentContext != null
        ? new ValueTask<IGitHubContext>(_currentContext)
        : new ValueTask<IGitHubContext>(CreateContextAsync(gitFolderPath));
    }
    finally
    {
      _lock.Release();
    }
  }

  private async Task<IGitHubContext> CreateContextAsync(string gitFolderPath)
  {
    (var repositoryOwner, var repositoryName) = GitHubContextFactory.GetRepositoryDetails(gitFolderPath);
    var repository = await _gitHubClient.Repository.Get(repositoryOwner, repositoryName);
    return new GitHubContext(_gitHubClient, repository);
  }

  private static (string RepositoryOwner, string RepositoryName) GetRepositoryDetails(string gitFolderPath)
  {
    using var repo = new GitRepository(gitFolderPath);
    var origin = repo.Network.Remotes["origin"];
    var url = origin.Url;
    var match = GitHubContextFactory.GetRepositoryUrlRegex().Match(url);
    if (!match.Success)
    {
      throw new InvalidOperationException("Not a valid GitHub URL.");
    }

    return (match.Groups["owner"].Value, match.Groups["repo"].Value);
  }

  [GeneratedRegex(@"github\.com[/:](?<owner>[^/]+?)/(?<repo>[^/.]+)(\.git)?$")]
  private static partial Regex GetRepositoryUrlRegex();
}
