using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Configuration;
using Wolfware.Moonlit.Plugins.Github.Extensions;
using Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Core.Middlewares;

internal sealed class GetGitInformation : ReleaseMiddleware<GetGitInformationConfiguration>
{
  private readonly IBranchesInformationProvider _branchesInformationProvider;
  private readonly ITagsInformationProvider _tagsInformationProvider;
  private readonly ICommitsInformationProvider _commitsInformationProvider;
  private readonly IPullRequestsInformationProvider _pullRequestsInformationProvider;
  private readonly IIssuesInformationProvider _issuesInformationProvider;

  public GetGitInformation(
    IBranchesInformationProvider branchesInformationProvider,
    ITagsInformationProvider tagsInformationProvider,
    ICommitsInformationProvider commitsInformationProvider,
    IPullRequestsInformationProvider pullRequestsInformationProvider,
    IIssuesInformationProvider issuesInformationProvider
  )
  {
    _branchesInformationProvider = branchesInformationProvider;
    _tagsInformationProvider = tagsInformationProvider;
    _commitsInformationProvider = commitsInformationProvider;
    _pullRequestsInformationProvider = pullRequestsInformationProvider;
    _issuesInformationProvider = issuesInformationProvider;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    GetGitInformationConfiguration configuration)
  {
    context.Logger.LogInformation("Collecting Git information from repository");

    var output = new Dictionary<string, object?>();

    output.AddRange(await this._branchesInformationProvider.GetInfo(
      context,
      configuration.Branches,
      context.CancellationToken
    ));

    output.AddRange(await this._tagsInformationProvider.GetInfo(
      context,
      configuration.Tags,
      context.CancellationToken
    ));

    output.AddRange(await this._commitsInformationProvider.GetInfo(
      context,
      configuration.Commits,
      context.CancellationToken
    ));

    output.AddRange(await this._pullRequestsInformationProvider.GetInfo(
      context,
      configuration.PullRequests,
      context.CancellationToken
    ));

    output.AddRange(await this._issuesInformationProvider.GetInfo(
      context,
      configuration.Issues,
      context.CancellationToken
    ));

    context.Logger.LogInformation("Git information collection completed.");
    return MiddlewareResult.Success(output);
  }
}
