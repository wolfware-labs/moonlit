using Octokit;

namespace Wolfware.Moonlit.Plugins.Github.Models;

public sealed class IssueDetails
{
  public int Number { get; set; }

  public string Title { get; set; } = string.Empty;

  public string Body { get; set; } = string.Empty;

  public ItemState State { get; set; }

  public DateTimeOffset CreatedAt { get; set; }

  public DateTimeOffset? UpdatedAt { get; set; }

  public DateTimeOffset? ClosedAt { get; set; }

  public int PullRequestNumber { get; set; }
}
