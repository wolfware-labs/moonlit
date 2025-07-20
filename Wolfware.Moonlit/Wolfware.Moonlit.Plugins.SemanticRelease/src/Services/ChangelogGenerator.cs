using Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public sealed class ChangelogGenerator : IChangelogGenerator
{
  public ChangelogCategory[] GenerateChangelog(ConventionalCommit[] commits)
  {
    throw new NotImplementedException();
  }
}
