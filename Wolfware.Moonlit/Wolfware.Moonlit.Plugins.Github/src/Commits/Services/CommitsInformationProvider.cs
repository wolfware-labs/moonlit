using Octokit;
using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Configuration;
using Wolfware.Moonlit.Plugins.Github.Commits.Models;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Services;

public sealed class CommitsInformationProvider : ICommitsInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public CommitsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task PopulateFetchContext(ReleaseContext releaseContext,
    CommitsInformationFetchConfiguration fetchConfiguration,
    FetchContext fetchContext)
  {
    if (fetchConfiguration.Strategy == CommitsFetchStrategy.None)
    {
      return;
    }

    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(releaseContext);
    var commitsContext = new CommitsFetchContext();
    if (fetchConfiguration.Strategy == CommitsFetchStrategy.All)
    {
      var commits = await gitHubContext.GetCommits(new CommitRequest());
      commitsContext.Commits = commits
        .Select(commit => new GitCommit
        {
          Sha = commit.Sha,
          Message = commit.Commit.Message,
          Author = commit.Author?.Login ?? commit.Commit.Author.Name,
          Date = commit.Commit.Author.Date
        })
        .ToArray();
    }
    else if (fetchConfiguration.Strategy == CommitsFetchStrategy.FromLatestTag)
    {
      if (fetchContext.Tags?.LatestTag is not { } latestTag)
      {
        throw new InvalidOperationException("Latest tag is required for 'FromLatestTag' strategy.");
      }

      var commits = await gitHubContext.GetCommits(new CommitRequest {Sha = latestTag.CommitSha});
      commitsContext.Commits = commits
        .Select(commit => new GitCommit
        {
          Sha = commit.Sha,
          Message = commit.Commit.Message,
          Author = commit.Author?.Login ?? commit.Commit.Author.Name,
          Date = commit.Commit.Author.Date
        })
        .ToArray();
    }
  }
}
