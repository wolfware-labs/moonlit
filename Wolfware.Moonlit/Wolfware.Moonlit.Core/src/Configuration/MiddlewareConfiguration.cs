namespace Wolfware.Moonlit.Core.Configuration;

public sealed class MiddlewareConfiguration
{
  public string Plugin { get; set; } = string.Empty;

  public string Name { get; set; } = string.Empty;

  public IReadOnlyDictionary<string, string> Configuration { get; set; } = new Dictionary<string, string>();
}
