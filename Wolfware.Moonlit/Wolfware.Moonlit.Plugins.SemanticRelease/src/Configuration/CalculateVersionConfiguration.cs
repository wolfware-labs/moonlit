using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public class CalculateVersionConfiguration
{
  public string InitialVersion { get; set; } = "1.0.0";

  public string? BaseVersion { get; set; }

  public string Branch { get; set; } = string.Empty;

  public ConventionalCommit[]? Commits { get; set; }

  public ConventionalCommitsAnalyzerConfiguration ConventionalCommitRules { get; set; } =
    ConventionalCommitsAnalyzerConfiguration.CreateDefault();

  public Dictionary<string, string> PrereleaseMappings { get; set; } = new();
}
