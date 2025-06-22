namespace Wolfware.Moonlit.Core.Configuration;

public sealed class StageConfiguration
{
  public string Name { get; set; } = string.Empty;

  public MiddlewareConfiguration[] Middlewares { get; set; } = [];
}
