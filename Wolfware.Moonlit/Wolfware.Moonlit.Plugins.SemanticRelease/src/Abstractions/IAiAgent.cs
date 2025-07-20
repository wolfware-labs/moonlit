using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;

public interface IAiAgent
{
  Task<ConventionalCommit[]> FilterOutNonUserFacingCommits(ConventionalCommit[] commits);

  Task<ConventionalCommit[]> RefineCommitsSummary(ConventionalCommit[] commits);
}
