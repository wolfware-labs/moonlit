namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public sealed class ConventionalCommit
{
  public string Type { get; set; } = string.Empty;

  public string? Scope { get; set; }

  public string Description { get; set; } = string.Empty;

  public bool IsBreakingChange { get; set; }

  public string? FullMessage { get; set; }

  public DateTimeOffset Date { get; set; }

  public string Sha { get; set; } = string.Empty;
}
