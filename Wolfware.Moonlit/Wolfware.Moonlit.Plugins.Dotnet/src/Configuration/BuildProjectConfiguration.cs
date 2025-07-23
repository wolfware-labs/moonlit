namespace Wolfware.Moonlit.Plugins.Dotnet.Configuration;

public sealed class BuildProjectConfiguration
{
  public string Project { get; set; } = string.Empty;

  public string? Version { get; set; }

  public string? AssemblyVersion { get; set; }

  public string? FileVersion { get; set; }

  public string? InformationalVersion { get; set; }

  public string Configuration { get; set; } = "Release";

  public bool NoRestore { get; set; } = false;
}
