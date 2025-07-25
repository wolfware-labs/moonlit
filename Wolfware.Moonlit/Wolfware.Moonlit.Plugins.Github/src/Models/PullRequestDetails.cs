﻿using Octokit;

namespace Wolfware.Moonlit.Plugins.Github.Models;

public sealed class PullRequestDetails
{
  public int Number { get; set; }

  public string Title { get; set; } = string.Empty;

  public string Body { get; set; } = string.Empty;

  public ItemState State { get; set; }

  public DateTimeOffset CreatedAt { get; set; }

  public DateTimeOffset UpdatedAt { get; set; }

  public DateTimeOffset? MergedAt { get; set; }

  public string MergeCommitSha { get; set; } = string.Empty;
}
