using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;

public interface IChangelogGenerator
{
  public ChangelogCategory[] GenerateChangelog(ConventionalCommit[] commits);
}
