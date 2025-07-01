namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public class CommitAnalysis
{
  public ConventionalCommit Commit { get; set; } = new();

  public VersionBumpType BumpType { get; set; }

  public ReleaseRule? MatchedRule { get; set; }
}
