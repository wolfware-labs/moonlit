namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class CalculateVersionConfiguration
{
  public string BaseVersion { get; set; } = "0.0.0";

  public string[] Commits { get; set; } = [];

  public ReleaseConfiguration Release = ReleaseConfiguration.CreateDefault();
}
