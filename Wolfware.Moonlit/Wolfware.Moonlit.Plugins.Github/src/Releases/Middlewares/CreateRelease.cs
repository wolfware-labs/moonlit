using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Releases.Configuration;
using Wolfware.Moonlit.Plugins.Github.Releases.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Releases.Middlewares;

public sealed class CreateRelease : ReleaseMiddleware<CreateReleaseConfiguration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public CreateRelease(IGitHubContextProvider gitHubContextProvider)
  {
    this._gitHubContextProvider = gitHubContextProvider;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    CreateReleaseConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(configuration.Name) || string.IsNullOrWhiteSpace(configuration.Tag))
    {
      return MiddlewareResult.Failure("Release name or tag is not specified.");
    }

    if (string.IsNullOrWhiteSpace(configuration.Body) &&
        (configuration.Changelog == null || configuration.Changelog.Count == 0))
    {
      return MiddlewareResult.Failure("Release body is not specified and no changelog entries are provided.");
    }

    context.Logger.LogInformation("Creating release '{ReleaseName}' with tag '{TagName}'.", configuration.Name,
      configuration.Tag);

    var release = new NewRelease(configuration.Tag)
    {
      Name = configuration.Name,
      Body = configuration.Body ?? CreateMarkdown(configuration.Changelog!),
      Draft = configuration.Draft,
      Prerelease = configuration.PreRelease
    };

    try
    {
      var gitHubContext = await this._gitHubContextProvider.GetCurrentContext(context);
      var createdRelease = await gitHubContext.CreateRelease(release);
      context.Logger.LogInformation("Release created successfully: {ReleaseUrl}", createdRelease.HtmlUrl);
      return MiddlewareResult.Success(output =>
      {
        output.Add("ReleaseUrl", createdRelease.HtmlUrl);
      });
    }
    catch (Exception)
    {
      return MiddlewareResult.Failure("Failed to create release.");
    }
  }

  private string CreateMarkdown(Dictionary<string, ChangelogEntry[]> changelog)
  {
    var markdown = new System.Text.StringBuilder();

    foreach (var entry in changelog)
    {
      markdown.AppendLine($"## {entry.Key}");
      foreach (var item in entry.Value)
      {
        markdown.AppendLine(
          $"- {item.Description} ([{item.Sha[..7]}]({string.Empty}/commit/{item.Sha})");
      }

      markdown.AppendLine();
    }

    return markdown.ToString();
  }
}
