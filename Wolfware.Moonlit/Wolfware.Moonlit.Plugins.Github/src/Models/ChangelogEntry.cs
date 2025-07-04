namespace Wolfware.Moonlit.Plugins.Github.Models;

public sealed class ChangelogEntry
{
  public string Sha { get; set; } = string.Empty;

  public string Description { get; set; } = string.Empty;
}
