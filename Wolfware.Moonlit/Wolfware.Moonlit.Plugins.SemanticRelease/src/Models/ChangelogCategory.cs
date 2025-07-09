namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public sealed class ChangelogCategory
{
  public string Name { get; set; } = string.Empty;

  public string Summary { get; set; } = string.Empty;

  public ChangelogEntry[] Entries { get; set; } = [];
}
