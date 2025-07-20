namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public record CommitSummary
{
  public string Sha { get; set; } = string.Empty;

  public string Summary { get; set; } = string.Empty;
}
