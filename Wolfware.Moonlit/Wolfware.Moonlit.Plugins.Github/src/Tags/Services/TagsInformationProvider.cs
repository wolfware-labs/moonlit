using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.Tags.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Configuration;
using Wolfware.Moonlit.Plugins.Github.Tags.Models;
using Wolfware.Moonlit.Plugins.Pipeline;
using GitTag = Wolfware.Moonlit.Plugins.Github.Tags.Models.GitTag;

namespace Wolfware.Moonlit.Plugins.Github.Tags.Services;

public sealed class TagsInformationProvider : ITagsInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public TagsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task PopulateFetchContext(
    ReleaseContext releaseContext,
    TagsInformationFetchConfiguration fetchConfiguration,
    FetchContext fetchContext
  )
  {
    if (fetchConfiguration.Strategy == TagsFetchStrategy.None)
    {
      return;
    }

    var gitHubContext = await this._gitHubContextProvider.GetCurrentContext(releaseContext);
    var tagsContext = new TagsFetchContext();
    var tags = await gitHubContext.GetTags(new ApiOptions());
    var filteredTags = tags;
    if (!string.IsNullOrEmpty(fetchConfiguration.FilterPattern))
    {
      var filterRegex = new Regex(fetchConfiguration.FilterPattern, RegexOptions.IgnoreCase);
      filteredTags = tags.Where(tag => filterRegex.IsMatch(tag.Name)).ToList();
    }

    if (fetchConfiguration.Strategy == TagsFetchStrategy.All)
    {
      releaseContext.Logger.LogInformation(
        "Fetching all tags matching the filter pattern: {FilterPattern}",
        fetchConfiguration.FilterPattern
      );
      tagsContext.Tags = filteredTags
        .Select(tag => new GitTag {Name = tag.Name, CommitSha = ((GitHubCommit)tag.Commit).Sha})
        .ToArray();
    }
    else if (fetchConfiguration.Strategy == TagsFetchStrategy.Latest)
    {
      releaseContext.Logger.LogInformation(
        "Fetching the latest tag matching the filter pattern: {FilterPattern}",
        fetchConfiguration.FilterPattern
      );
      var latestTag = filteredTags
        .OrderByDescending(tag => ((GitHubCommit)tag.Commit).Commit.Author.Date)
        .FirstOrDefault();
      if (latestTag != null)
      {
        tagsContext.LatestTag = new GitTag {Name = latestTag.Name, CommitSha = ((GitHubCommit)latestTag.Commit).Sha};
      }
    }

    fetchContext.Tags = tagsContext;
  }
}
