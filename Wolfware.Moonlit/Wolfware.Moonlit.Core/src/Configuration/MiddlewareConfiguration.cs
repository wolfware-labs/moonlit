namespace Wolfware.Moonlit.Core.Configuration;

public sealed class MiddlewareConfiguration
{
  public string Plugin { get; set; } = string.Empty;

  public string Name { get; set; } = string.Empty;

  public Dictionary<string, string?> Configuration { get; set; } = new();
}
