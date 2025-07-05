using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Github.Branches.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Commits.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Configuration;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
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

    var fetchContext = new FetchContext();
    await this._branchesInformationProvider.PopulateFetchContext(context, configuration.Branches, fetchContext);
    await this._tagsInformationProvider.PopulateFetchContext(context, configuration.Tags, fetchContext);
    await this._commitsInformationProvider.PopulateFetchContext(context, configuration.Commits, fetchContext);
    await this._pullRequestsInformationProvider.PopulateFetchContext(context, configuration.PullRequests, fetchContext);
    await this._issuesInformationProvider.PopulateFetchContext(context, configuration.Issues, fetchContext);

    context.Logger.LogInformation("Git information collection completed.");
    return MiddlewareResult.Success(output =>
    {
      if (fetchContext.Branches != null)
      {
        output.Add("Branches", fetchContext.Branches);
      }

      if (fetchContext.Tags != null)
      {
        output.Add("Tags", fetchContext.Tags);
      }

      if (fetchContext.Commits != null)
      {
        output.Add("Commits", fetchContext.Commits);
      }

      if (fetchContext.PullRequests != null)
      {
        output.Add("PullRequests", fetchContext.PullRequests);
      }

      if (fetchContext.Issues != null)
      {
        output.Add("Issues", fetchContext.Issues);
      }
    });
  }
}
