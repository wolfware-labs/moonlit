namespace Wolfware.Moonlit.Plugins.Github.Models;

public sealed class ChangelogCategory
{
  public string Heading { get; set; } = string.Empty;

  public string Emoji { get; set; } = string.Empty;

  public string Summary { get; set; } = string.Empty;

  public ChangelogEntry[] Entries { get; set; } = [];
}
