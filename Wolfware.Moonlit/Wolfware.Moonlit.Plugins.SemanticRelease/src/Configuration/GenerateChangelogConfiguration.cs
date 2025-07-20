using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class GenerateChangelogConfiguration
{
  public ConventionalCommit[]? Commits { get; set; }

  public string? OpenAiKey { get; set; } = string.Empty;

  public bool FilterNonUserFacingCommits { get; set; } = true;

  public bool RefineCommitMessages { get; set; } = true;
}
