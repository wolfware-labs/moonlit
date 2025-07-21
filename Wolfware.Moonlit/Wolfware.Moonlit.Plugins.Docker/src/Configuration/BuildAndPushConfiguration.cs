namespace Wolfware.Moonlit.Plugins.Docker.Configuration;

public sealed class BuildAndPushConfiguration
{
  public string? Builder { get; set; }

  public string[] Tags { get; set; } = [];

  public string? File { get; set; }

  public string Context { get; set; } = ".";

  public bool Push { get; set; } = true;

  public string[] BuildArgs { get; set; } = [];

  public Dictionary<string, string> Labels { get; set; } = [];

  public string[] Platforms { get; set; } = [];

  public bool NoCache { get; set; } = false;

  public bool Pull { get; set; } = false;

  public string[] CacheFrom { get; set; } = [];

  public string[] CacheTo { get; set; } = [];
}
