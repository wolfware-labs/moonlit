using Wolfware.Moonlit.Plugins.Github.Issues.Models;

namespace Wolfware.Moonlit.Plugins.Github.Issues.Configuration;

public sealed class IssuesInformationFetchConfiguration
{
  public IssuesFetchStrategy Strategy { get; set; } = IssuesFetchStrategy.FromAvailablePullRequests;
}
