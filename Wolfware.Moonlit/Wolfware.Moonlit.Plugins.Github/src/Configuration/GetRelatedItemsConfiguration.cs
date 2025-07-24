using Wolfware.Moonlit.Plugins.Github.Models;

namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public sealed class GetRelatedItemsConfiguration
{
  public CommitDetails[] Commits { get; set; } = [];

  public bool IncludePullRequests { get; set; } = true;

  public bool IncludeIssues { get; set; } = true;
}
