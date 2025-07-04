using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Github.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class CreateRelease : ReleaseMiddleware<CreateRelease.Configuration>
{
  private readonly GitHubConfiguration _gitHubConfiguration;
  private readonly IGitHubClient _gitHubClient;

  public sealed class Configuration
  {
    public string Name { get; set; } = string.Empty;

    public string Tag { get; set; } = string.Empty;

    public string? Body { get; set; }

    public Dictionary<string, ChangelogEntry[]>? Changelog { get; set; }

    public bool Draft { get; set; } = false;

    public bool PreRelease { get; set; } = false;
  }

  public CreateRelease(IOptions<GitHubConfiguration> gitHubConfiguration, IGitHubClient gitHubClient)
  {
    _gitHubConfiguration = gitHubConfiguration.Value;
    _gitHubClient = gitHubClient;
  }

  public override async Task<MiddlewareResult> ExecuteAsync(PipelineContext context, Configuration configuration)
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
      var createdRelease = await _gitHubClient.Repository.Release.Create(
        this._gitHubConfiguration.Owner,
        this._gitHubConfiguration.Repository,
        release
      );
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
