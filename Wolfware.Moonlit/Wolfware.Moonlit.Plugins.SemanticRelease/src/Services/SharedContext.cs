using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public sealed class SharedContext
{
  public ConventionalCommit[] Commits { get; set; } = [];
}
