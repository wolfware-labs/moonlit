namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public class ReleaseRule
{
  public string Type { get; set; } = string.Empty;

  public string Scope { get; set; } = string.Empty;

  public VersionBumpType Release { get; set; }

  public bool Matches(ConventionalCommit commit)
  {
    if (!string.IsNullOrEmpty(Type) && !string.Equals(commit.Type, Type, StringComparison.OrdinalIgnoreCase))
    {
      return false;
    }

    if (!string.IsNullOrEmpty(Scope) && !string.Equals(commit.Scope, Scope, StringComparison.OrdinalIgnoreCase))
    {
      return false;
    }

    return true;
  }
}
