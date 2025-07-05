using Wolfware.Moonlit.Plugins.Github.Branches.Models;
using Wolfware.Moonlit.Plugins.Github.Commits.Models;
using Wolfware.Moonlit.Plugins.Github.Issues.Models;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Models;
using Wolfware.Moonlit.Plugins.Github.Tags.Models;

namespace Wolfware.Moonlit.Plugins.Github.Core.Models;

public sealed class FetchContext
{
  public BranchesFetchContext? Branches { get; set; }

  public TagsFetchContext? Tags { get; set; }

  public CommitsFetchContext? Commits { get; set; }

  public PullRequestsFetchContext? PullRequests { get; set; }

  public IssuesFetchContext? Issues { get; set; }
}
