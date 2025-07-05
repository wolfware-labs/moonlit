namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public class GetLatestTagConfiguration
{
  public string? Prefix { get; set; } = null;

  public string? Suffix { get; set; } = null;

  public string Pattern { get; set; } = "[0-9]+.[0-9]+.[0-9]+";
}
