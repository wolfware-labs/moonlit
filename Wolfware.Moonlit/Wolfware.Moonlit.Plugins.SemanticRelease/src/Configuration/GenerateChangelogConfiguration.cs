using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class GenerateChangelogConfiguration
{
  public ConventionalCommit[]? Commits { get; set; }

  public bool FilterNonUserFacingCommits { get; set; } = false;

  public bool RefineCommitsSummary { get; set; } = false;
}
