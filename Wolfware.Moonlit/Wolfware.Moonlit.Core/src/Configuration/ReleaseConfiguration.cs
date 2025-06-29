namespace Wolfware.Moonlit.Core.Configuration;

public sealed class ReleaseConfiguration
{
  public string Name { get; init; } = string.Empty;

  public Dictionary<string, string> Arguments { get; init; } = [];

  public Dictionary<string, string> Variables { get; init; } = [];

  public PluginConfiguration[] Plugins { get; init; } = [];

  public Dictionary<string, StepConfiguration[]> Stages { get; init; } = [];
}
