namespace Wolfware.Moonlit.Plugins.Dotnet.Configuration;

public sealed class DotnetConfiguration
{
  public string NugetSource { get; set; } = "https://api.nuget.org/v3/index.json";

  public string NugetApiKey { get; set; } = string.Empty;
}
