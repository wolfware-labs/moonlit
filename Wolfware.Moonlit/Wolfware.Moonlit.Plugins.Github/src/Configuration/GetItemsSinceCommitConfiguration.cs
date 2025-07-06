namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public sealed class GetItemsSinceCommitConfiguration
{
  public string? Sha { get; set; }

  public bool IncludePullRequests { get; set; } = true;

  public bool IncludeIssues { get; set; } = true;

  public bool IncludeCommits { get; set; } = true;
}
