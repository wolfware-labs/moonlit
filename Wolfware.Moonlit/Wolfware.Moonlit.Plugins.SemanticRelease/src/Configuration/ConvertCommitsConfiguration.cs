using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class ConvertCommitsConfiguration
{
  public Commit[] Commits { get; set; } = [];

  public string[]? IncludeScopes { get; set; }

  public string[]? ExcludeScopes { get; set; }

  public bool IncludeUnscoped { get; set; } = true;
}
