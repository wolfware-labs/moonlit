namespace Wolfware.Moonlit.Core.Configuration;

public sealed class StepConfiguration
{
  public string StepName { get; set; } = string.Empty;

  public string PluginName { get; set; } = string.Empty;

  public string MiddlewareName { get; set; } = string.Empty;

  public Dictionary<string, string?> Configuration { get; set; } = new();
}
