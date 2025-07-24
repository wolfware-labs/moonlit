using System.Text.RegularExpressions;
using LibGit2Sharp;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Git.Extensions;
using Wolfware.Moonlit.Plugins.Git.Services;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class GetLatestTag : ReleaseMiddleware<GetLatestTagConfiguration>
{
  private readonly ILogger<GetLatestTag> _logger;
  private readonly SharedContext _sharedContext;

  public GetLatestTag(ILogger<GetLatestTag> logger, SharedContext sharedContext)
  {
    _logger = logger;
    _sharedContext = sharedContext;
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    GetLatestTagConfiguration configuration)
  {
    var gitFolderPath = context.WorkingDirectory.GetGitFolderPath();
    using var repo = new Repository(gitFolderPath);

    var filterPattern =
      $"^{configuration.Prefix ?? string.Empty}{configuration.Pattern}{configuration.Suffix ?? string.Empty}$";

    var filterRegex = new Regex(filterPattern, RegexOptions.IgnoreCase);
    var latestTag = repo.Tags
      .Where(tag => filterRegex.IsMatch(tag.FriendlyName))
      .OrderByDescending(tag => tag.Target.Peel<Commit>()?.Committer.When)
      .FirstOrDefault();

    if (latestTag == null)
    {
      this._logger.LogWarning(
        "No tags found matching the filter pattern: {FilterPattern}",
        filterPattern
      );
      return Task.FromResult(MiddlewareResult.Success());
    }

    this._logger.LogInformation(
      "Latest tag found: {TagName}",
      latestTag.FriendlyName ?? "None"
    );

    this._sharedContext.LatestTagSha = latestTag.Target.Sha;

    var name = latestTag.FriendlyName?[(configuration.Prefix ?? string.Empty).Length..] ?? string.Empty;
    name = name[..^((configuration.Suffix ?? string.Empty).Length)];

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("name", name);
      output.Add("fullName", latestTag.FriendlyName);
      output.Add("commitSha", latestTag.Target.Sha);
    }));
  }
}
