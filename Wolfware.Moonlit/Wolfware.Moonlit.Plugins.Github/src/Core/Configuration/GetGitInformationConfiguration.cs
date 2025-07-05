using Wolfware.Moonlit.Plugins.Github.Branches.Configuration;
using Wolfware.Moonlit.Plugins.Github.Commits.Configuration;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;
using Wolfware.Moonlit.Plugins.Github.Tags.Configuration;

namespace Wolfware.Moonlit.Plugins.Github.Core.Configuration;

public class GetGitInformationConfiguration
{
  public BranchesInformationFetchConfiguration Branches { get; set; } = new();

  public TagsInformationFetchConfiguration Tags { get; set; } = new();

  public CommitsInformationFetchConfiguration Commits { get; set; } = new();

  public PullRequestsInformationFetchConfiguration PullRequests { get; set; } = new();

  public IssuesInformationFetchConfiguration Issues { get; set; } = new();
}
