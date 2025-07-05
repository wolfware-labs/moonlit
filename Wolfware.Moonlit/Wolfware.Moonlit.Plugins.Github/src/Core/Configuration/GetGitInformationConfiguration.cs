using Wolfware.Moonlit.Plugins.Github.Branches.Configuration;
using Wolfware.Moonlit.Plugins.Github.Commits.Configuration;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;
using Wolfware.Moonlit.Plugins.Github.Tags.Configuration;

namespace Wolfware.Moonlit.Plugins.Github.Core.Configuration;

public class GetGitInformationConfiguration
{
  public BranchesInformationFetchConfiguration? Branches { get; set; }

  public TagsInformationFetchConfiguration? Tags { get; set; }

  public CommitsInformationFetchConfiguration? Commits { get; set; }

  public PullRequestsInformationFetchConfiguration? PullRequests { get; set; }

  public IssuesInformationFetchConfiguration? Issues { get; set; }
}
