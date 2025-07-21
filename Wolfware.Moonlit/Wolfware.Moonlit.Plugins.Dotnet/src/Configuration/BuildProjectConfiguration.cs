namespace Wolfware.Moonlit.Plugins.Dotnet.Configuration;

public sealed class BuildProjectConfiguration
{
  public string Project { get; set; } = string.Empty;

  public string Configuration { get; set; } = "Release";
}
