﻿namespace Wolfware.Moonlit.Plugins.Github.Models;

public sealed class ChangelogCategory
{
  public string Name { get; set; } = string.Empty;

  public string Icon { get; set; } = string.Empty;

  public string Summary { get; set; } = string.Empty;

  public ChangelogEntry[] Entries { get; set; } = [];
}
