namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Models;

public sealed class PullRequestsFetchContext
{
  public PullRequestInformation[] PullRequests { get; set; } = [];
}
