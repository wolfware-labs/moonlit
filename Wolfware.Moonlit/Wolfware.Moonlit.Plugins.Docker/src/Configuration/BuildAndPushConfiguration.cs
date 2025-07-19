namespace Wolfware.Moonlit.Plugins.Docker.Configuration;

public sealed class BuildAndPushConfiguration
{
  public string[] Tags { get; set; } = [];

  public string? File { get; set; }

  public string Context { get; set; } = ".";

  public bool Push { get; set; } = false;

  public string[] BuildArgs { get; set; } = [];

  public string[] Labels { get; set; } = [];

  public string[] Platforms { get; set; } = [];

  public bool NoCache { get; set; } = false;

  public bool Pull { get; set; } = false;
}
