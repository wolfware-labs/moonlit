using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class CalculateVersionConfiguration
{
  public string BaseVersion { get; set; } = "0.0.0";

  public string Branch { get; set; } = string.Empty;

  public CommitMessage[] Commits { get; set; } = [];

  public ReleaseConfiguration Release = ReleaseConfiguration.CreateDefault();

  public Dictionary<string, string> PrereleaseMappings { get; set; } = new();
}
