namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public sealed class ReleaseRule
{
  public string? Type { get; set; }

  public string? Scope { get; set; }

  public VersionBumpType Release { get; set; }

  public bool Matches(ConventionalCommit commit)
  {
    return (string.IsNullOrEmpty(Type) || string.Equals(commit.Type, Type, StringComparison.OrdinalIgnoreCase)) &&
           (string.IsNullOrEmpty(Scope) || string.Equals(commit.Scope, Scope, StringComparison.OrdinalIgnoreCase));
  }
}
