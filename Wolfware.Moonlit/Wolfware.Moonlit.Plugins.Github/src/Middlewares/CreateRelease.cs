using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Github.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Github.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class CreateRelease : ReleaseMiddleware<CreateReleaseConfiguration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;
  private IGitHubContext? _gitHubContext;

  public CreateRelease(IGitHubContextProvider gitHubContextProvider)
  {
    this._gitHubContextProvider = gitHubContextProvider;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    CreateReleaseConfiguration configuration)
  {
    var validationResult = CreateRelease.ValidateConfiguration(configuration);
    if (validationResult != null)
    {
      return validationResult;
    }

    this._gitHubContext = await this._gitHubContextProvider.GetCurrentContext(context);

    var tag = await CreateGitHubTag(context, configuration).ConfigureAwait(false);

    var release = await CreateGitHubRelease(context, configuration).ConfigureAwait(false);
    if (configuration.PullRequests is {Length: > 0})
    {
      await this.AnnotatePullRequests(context, release, configuration.PullRequests);
    }

    if (configuration.Issues is {Length: > 0})
    {
      await this.AnnotateIssues(context, release, configuration.Issues);
    }

    return MiddlewareResult.Success(output =>
    {
      output.Add("ReleaseName", release.Name);
      output.Add("ReleaseUrl", release.HtmlUrl);
    });
  }

  private static MiddlewareResult? ValidateConfiguration(CreateReleaseConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(configuration.Name) ||
        string.IsNullOrWhiteSpace(configuration.Commit) ||
        string.IsNullOrWhiteSpace(configuration.Tag))
    {
      return MiddlewareResult.Failure("Release name, commit (SHA), and tag must be specified.");
    }

    if (string.IsNullOrWhiteSpace(configuration.Body) &&
        (configuration.Changelog == null || configuration.Changelog.Count == 0))
    {
      return MiddlewareResult.Failure("Release body is not specified and no changelog entries are provided.");
    }

    return null;
  }

  private async Task<GitTag> CreateGitHubTag(ReleaseContext context, CreateReleaseConfiguration configuration)
  {
    context.Logger.LogInformation("Creating tag '{TagName}' for commit '{CommitSha}'.", configuration.Tag,
      configuration.Commit);

    var tag = await this._gitHubContext!.CreateTag(configuration.Tag, configuration.Commit, configuration.Name);

    context.Logger.LogInformation("Tag created successfully: {TagUrl}", tag.Url);
    return tag;
  }

  private async Task<Release> CreateGitHubRelease(ReleaseContext context, CreateReleaseConfiguration configuration)
  {
    context.Logger.LogInformation("Creating release '{ReleaseName}' with tag '{TagName}'.", configuration.Name,
      configuration.Tag);

    var release = new NewRelease(configuration.Tag)
    {
      Name = configuration.Name,
      Body = configuration.Body ?? CreateMarkdown(this._gitHubContext!.Repository.HtmlUrl, configuration.Changelog!),
      Draft = configuration.Draft,
      Prerelease = configuration.PreRelease
    };


    var createdRelease = await this._gitHubContext!.CreateRelease(release);
    context.Logger.LogInformation("Release created successfully: {ReleaseUrl}", createdRelease.HtmlUrl);
    return createdRelease;
  }

  private string CreateMarkdown(string repositoryUrl, Dictionary<string, ChangelogEntry[]> changelog)
  {
    var markdown = new System.Text.StringBuilder();

    foreach (var entry in changelog)
    {
      markdown.AppendLine($"## {entry.Key}");
      foreach (var item in entry.Value)
      {
        markdown.AppendLine(
          $"- {item.Description} ([{item.Sha[..7]}]({repositoryUrl}/commit/{item.Sha}))");
      }

      markdown.AppendLine();
    }

    return markdown.ToString();
  }

  private async Task AnnotatePullRequests(ReleaseContext context, Release release, PullRequestDetails[] pullRequests)
  {
    foreach (var pr in pullRequests)
    {
      context.Logger.LogInformation("Annotating pull request #{PullRequestNumber} with release {ReleaseName}.",
        pr.Number, release.Name);
      await this._gitHubContext!.CommentOnPullRequest(
        pr.Number,
        CreateRelease.GetReleaseComment(release.Name, release.HtmlUrl)
      );
    }
  }

  private async Task AnnotateIssues(ReleaseContext context, Release release, IssueDetails[] issues)
  {
    foreach (var issue in issues)
    {
      context.Logger.LogInformation("Annotating issue #{IssueNumber} with release {ReleaseName}.",
        issue.Number, release.Name);
      await this._gitHubContext!.CommentOnIssue(
        issue.Number,
        CreateRelease.GetReleaseComment(release.Name, release.HtmlUrl)
      );
    }
  }

  private static string GetReleaseComment(string releaseName, string releaseUrl) =>
    $"""
     :rocket: **New Release Published!**

     :tada: A new version of the project has just been released!

     **:bookmark: Link:** [`{releaseName}`]({releaseUrl})
     """;
}
