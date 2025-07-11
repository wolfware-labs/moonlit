namespace Wolfware.Moonlit.Plugins.Nuget.Configuration;

public class NugetConfiguration
{
  public string Source { get; set; } = "https://api.nuget.org/v3/index.json";

  public string ApiKey { get; set; } = string.Empty;
}
