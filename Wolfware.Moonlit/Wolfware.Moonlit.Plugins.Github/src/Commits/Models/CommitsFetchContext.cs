namespace Wolfware.Moonlit.Plugins.Github.Commits.Models;

public sealed class CommitsFetchContext
{
  public GitCommit[] Commits { get; set; } = [];
}
