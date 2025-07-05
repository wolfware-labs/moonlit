namespace Wolfware.Moonlit.Plugins.Github.Tags.Models;

public class TagsFetchContext
{
  public GitTag LatestTag { get; set; } = new();

  public GitTag[] Tags { get; set; } = [];
}
