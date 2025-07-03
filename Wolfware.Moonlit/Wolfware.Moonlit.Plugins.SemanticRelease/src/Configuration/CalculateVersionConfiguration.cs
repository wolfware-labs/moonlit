using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class CalculateVersionConfiguration
{
  public string BaseVersion { get; set; } = "0.0.0";

  public CommitMessage[] Commits { get; set; } = [];

  public ReleaseConfiguration Release = ReleaseConfiguration.CreateDefault();
}
