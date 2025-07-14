using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class GetLatestTag : ReleaseMiddleware<GetLatestTagConfiguration>
{
  private readonly IGitHubContextProvider _contextProvider;
  private readonly ILogger<GetLatestTag> _logger;

  public GetLatestTag(IGitHubContextProvider contextProvider, ILogger<GetLatestTag> logger)
  {
    _contextProvider = contextProvider;
    _logger = logger;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    GetLatestTagConfiguration configuration)
  {
    var gitHubContext = await _contextProvider.GetCurrentContext(context);

    var filterPattern =
      $"^{configuration.Prefix ?? string.Empty}{configuration.Pattern}{configuration.Suffix ?? string.Empty}$";

    var tags = await gitHubContext.GetTags(new ApiOptions());
    var filterRegex = new Regex(filterPattern, RegexOptions.IgnoreCase);
    var latestTag = tags
      .Where(tag => filterRegex.IsMatch(tag.Name))
      .OrderByDescending(tag => ((GitHubCommit)tag.Commit).Commit.Author.Date)
      .FirstOrDefault();

    if (latestTag == null)
    {
      this._logger.LogWarning(
        "No tags found matching the filter pattern: {FilterPattern}",
        filterPattern
      );
      return MiddlewareResult.Success();
    }

    this._logger.LogInformation(
      "Latest tag found: {TagName}",
      latestTag.Name ?? "None"
    );
    return MiddlewareResult.Success(output =>
    {
      output.Add("Name", latestTag.Name ?? string.Empty);
      output.Add("CommitSha", ((GitHubCommit)latestTag.Commit).Sha);
    });
  }
}
