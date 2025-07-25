﻿using Wolfware.Moonlit.Plugins.Github.Models;

namespace Wolfware.Moonlit.Plugins.Github.Configuration;

public sealed class CreateReleaseConfiguration
{
  public string Name { get; set; } = string.Empty;

  public string Tag { get; set; } = string.Empty;

  public string? Label { get; set; }

  public string? Body { get; set; }

  public ChangelogCategory[] Changelog { get; set; } = [];

  public bool Draft { get; set; } = false;

  public bool PreRelease { get; set; } = false;

  public PullRequestDetails[]? PullRequests { get; set; }

  public IssueDetails[]? Issues { get; set; }
}
