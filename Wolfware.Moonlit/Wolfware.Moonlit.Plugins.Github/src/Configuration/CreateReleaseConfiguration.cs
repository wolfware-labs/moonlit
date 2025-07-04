namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public sealed class CreateReleaseConfiguration
{
  public string Name { get; set; } = string.Empty;

  public string Tag { get; set; } = string.Empty;

  public string Body { get; set; } = string.Empty;

  public bool Draft { get; set; } = false;

  public bool PreRelease { get; set; } = false;
}
