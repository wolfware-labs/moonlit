namespace Wolfware.Moonlit.Plugins.Github.Core.Models;

public sealed class RepositoryDetails
{
  public string Owner { get; set; } = string.Empty;

  public string Name { get; set; } = string.Empty;

  public string CurrentBranch { get; set; } = string.Empty;
}
