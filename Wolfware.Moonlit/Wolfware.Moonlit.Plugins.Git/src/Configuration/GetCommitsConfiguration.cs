namespace Wolfware.Moonlit.Plugins.Git.Configuration;

public sealed class GetCommitsConfiguration
{
  public string? SinceSha { get; set; }

  public bool UseSharedContext { get; set; } = true;
}
