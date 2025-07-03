namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public sealed class ChangelogEntry
{
  public string Sha { get; set; } = string.Empty;

  public string Description { get; set; } = string.Empty;
}
