namespace Wolfware.Moonlit.Core.Configuration;

public sealed class ReleaseConfiguration
{
  public string Name { get; init; } = string.Empty;

  public IReadOnlyDictionary<string, string> Arguments { get; init; } = new Dictionary<string, string>();

  public IReadOnlyDictionary<string, string> Variables { get; init; } = new Dictionary<string, string>();

  public PluginConfiguration[] Plugins { get; init; } = [];

  public StageConfiguration[] Stages { get; init; } = [];
}
