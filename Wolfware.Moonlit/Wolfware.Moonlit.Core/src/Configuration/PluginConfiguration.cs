namespace Wolfware.Moonlit.Core.Configuration;

public sealed class PluginConfiguration
{
  public string Name { get; set; } = string.Empty;

  public string Url { get; set; } = string.Empty;

  public Dictionary<string, string> Configuration { get; set; } = [];
}
