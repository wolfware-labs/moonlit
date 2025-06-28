namespace Wolfware.Moonlit.Core.Configuration;

public sealed class PluginConfiguration
{
  public string Name { get; set; } = string.Empty;

  public Uri Url { get; set; } = new("file://plugin.dll");

  public Dictionary<string, string?> Configuration { get; set; } = [];
}
