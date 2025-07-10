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
  private readonly ILogger<CreateRelease> _logger;
  private IGitHubContext? _gitHubContext;

  public CreateRelease(IGitHubContextProvider gitHubContextProvider, ILogger<CreateRelease> logger)
  {
    this._gitHubContextProvider = gitHubContextProvider;
    _logger = logger;
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
    var release = await CreateGitHubRelease(context, configuration).ConfigureAwait(false);
    if (configuration.PullRequests is {Length: > 0})
    {
      await this.AnnotatePullRequests(release, configuration.PullRequests);
      if (!string.IsNullOrWhiteSpace(configuration.Label))
      {
        await this.LabelPullRequests(configuration.Label, configuration.PullRequests);
      }
    }

    if (configuration.Issues is {Length: > 0})
    {
      await this.AnnotateIssues(release, configuration.Issues);
      if (!string.IsNullOrEmpty(configuration.Label))
      {
        await this.LabelIssues(configuration.Label, configuration.Issues);
      }
    }

    return MiddlewareResult.Success(output =>
    {
      output.Add("Name", release.Name);
      output.Add("Url", release.HtmlUrl);
    });
  }

  private static MiddlewareResult? ValidateConfiguration(CreateReleaseConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(configuration.Name) || string.IsNullOrWhiteSpace(configuration.Tag))
    {
      return MiddlewareResult.Failure("Release name or tag is not specified.");
    }

    if (string.IsNullOrWhiteSpace(configuration.Body) && configuration.Changelog.Length == 0)
    {
      return MiddlewareResult.Failure("Release body is not specified and no changelog entries are provided.");
    }

    return null;
  }

  private async Task<Release> CreateGitHubRelease(ReleaseContext context, CreateReleaseConfiguration configuration)
  {
    this._logger.LogInformation("Creating release '{ReleaseName}' with tag '{TagName}'.", configuration.Name,
      configuration.Tag);

    var release = new NewRelease(configuration.Tag)
    {
      Name = configuration.Name,
      Body = configuration.Body ??
             await CreateMarkdown(this._gitHubContext!.Repository.HtmlUrl, configuration.Changelog),
      Draft = configuration.Draft,
      Prerelease = configuration.PreRelease
    };


    var createdRelease = await this._gitHubContext!.CreateRelease(release);
    this._logger.LogInformation("Release created successfully: {ReleaseUrl}", createdRelease.HtmlUrl);
    return createdRelease;
  }

  private async Task<string> CreateMarkdown(string repositoryUrl, ChangelogCategory[] changelog)
  {
    var pullRequests = await this._gitHubContext!.GetPullRequests(new PullRequestRequest {State = ItemStateFilter.All});
    var markdown = new System.Text.StringBuilder();

    foreach (var category in changelog)
    {
      markdown.AppendLine($"### {category.Icon} {category.Name}");
      markdown.AppendLine($"#### {category.Summary}");
      foreach (var item in category.Entries)
      {
        markdown.Append($"- {item.Description}");
        var pullRequest = pullRequests.FirstOrDefault(pr => pr.MergeCommitSha == item.Sha);
        if (pullRequest != null)
        {
          markdown.Append($" ([#{pullRequest.Number}]({repositoryUrl}/pull/{pullRequest.Number}))");
        }

        markdown.AppendLine();
      }

      markdown.AppendLine();
    }

    return markdown.ToString();
  }

  private async Task AnnotatePullRequests(Release release, PullRequestDetails[] pullRequests)
  {
    foreach (var pr in pullRequests)
    {
      this._logger.LogInformation("Annotating pull request #{PullRequestNumber} with release {ReleaseName}.",
        pr.Number, release.Name);
      await this._gitHubContext!.CommentOnPullRequest(
        pr.Number,
        CreateRelease.GetReleaseComment(release.Name, release.HtmlUrl)
      );
    }
  }

  private async Task LabelPullRequests(string label, PullRequestDetails[] pullRequests)
  {
    foreach (var pr in pullRequests)
    {
      this._logger.LogInformation("Labeling pull request #{PullRequestNumber} with label {Label}.",
        pr.Number, label);
      await this._gitHubContext!.LabelPullRequest(pr.Number, label);
    }
  }

  private async Task AnnotateIssues(Release release, IssueDetails[] issues)
  {
    foreach (var issue in issues)
    {
      this._logger.LogInformation("Annotating issue #{IssueNumber} with release {ReleaseName}.",
        issue.Number, release.Name);
      await this._gitHubContext!.CommentOnIssue(
        issue.Number,
        CreateRelease.GetReleaseComment(release.Name, release.HtmlUrl)
      );
    }
  }

  public async Task LabelIssues(string label, IssueDetails[] issues)
  {
    foreach (var issue in issues)
    {
      this._logger.LogInformation("Labeling issue #{IssueNumber} with label {Label}.",
        issue.Number, label);
      await this._gitHubContext!.LabelIssue(issue.Number, label);
    }
  }

  private static string GetReleaseComment(string releaseName, string releaseUrl) =>
    $"""
     :rocket: **New Release Published!**

     :tada: A new version of the project has just been released!

     **:bookmark: Link:** [`{releaseName}`]({releaseUrl})
     """;
}
