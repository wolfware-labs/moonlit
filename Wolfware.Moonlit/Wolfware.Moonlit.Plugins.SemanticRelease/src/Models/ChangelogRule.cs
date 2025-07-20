namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public sealed class ChangelogRule
{
  public string? Type { get; set; }

  public bool IsBreakingChange { get; set; }

  public string Icon { get; set; } = ":sparkles:";

  public string Section { get; set; } = "Uncategorized";

  public string Summary { get; set; } = "Other changes";

  public bool Matches(ConventionalCommit commit)
  {
    return string.IsNullOrEmpty(Type) || string.Equals(commit.Type, Type, StringComparison.OrdinalIgnoreCase) &&
      commit.IsBreakingChange == IsBreakingChange;
  }
}
