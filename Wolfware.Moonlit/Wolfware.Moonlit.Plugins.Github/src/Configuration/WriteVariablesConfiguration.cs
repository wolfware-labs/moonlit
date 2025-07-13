namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public sealed class WriteVariablesConfiguration
{
  public Dictionary<string, string> Output { get; set; } = new();

  public Dictionary<string, string> Environment { get; set; } = new();
}
