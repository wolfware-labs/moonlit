namespace Wolfware.Moonlit.Core.Configuration;

public sealed class PluginConfiguration
{
  public string Name { get; set; } = string.Empty;

  public string Url { get; set; } = string.Empty;

  public IReadOnlyDictionary<string, string> Configuration { get; set; } = new Dictionary<string, string>();
}
