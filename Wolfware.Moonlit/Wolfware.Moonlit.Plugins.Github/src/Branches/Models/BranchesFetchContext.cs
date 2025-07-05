namespace Wolfware.Moonlit.Plugins.Github.Branches.Models;

public sealed class BranchesFetchContext
{
  public string CurrentBranch { get; set; } = string.Empty;

  public string[] Branches { get; set; } = [];
}
