namespace Wolfware.Moonlit.Plugins.Github.Models;

public sealed class CommitDetails
{
  public string Sha { get; set; } = string.Empty;

  public string Author { get; set; } = string.Empty;

  public string Email { get; set; } = string.Empty;

  public DateTimeOffset Date { get; set; }

  public string Message { get; set; } = string.Empty;
}
