namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public sealed record ConventionalCommit : CommitSummary
{
  public string Type { get; set; } = string.Empty;

  public string? Scope { get; set; }

  public string Body { get; set; } = string.Empty;

  public bool IsBreakingChange { get; set; }

  public string RawMessage { get; set; } = string.Empty;

  public DateTimeOffset Date { get; set; }
}
