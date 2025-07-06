namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public class WriteVariablesConfiguration
{
  public Dictionary<string, string> Output { get; set; } = new();

  public Dictionary<string, string> Environment { get; set; } = new();
}
