using Wolfware.Moonlit.Plugins.Github.PullRequests.Models;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;

public class PullRequestsInformationFetchConfiguration
{
  public PullRequestsFetchStrategy Strategy { get; set; } = PullRequestsFetchStrategy.FromAvailableCommits;
}
