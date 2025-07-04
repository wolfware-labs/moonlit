using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class CommitsAnalyzerConfiguration
{
  public bool BreakingChangesAlwaysMajor { get; set; } = true;

  public List<ReleaseRule> Rules { get; set; } = [];

  public static CommitsAnalyzerConfiguration CreateDefault()
  {
    return new CommitsAnalyzerConfiguration
    {
      Rules =
      [
        new ReleaseRule {Type = "feat", Release = VersionBumpType.Minor},
        new ReleaseRule {Type = "fix", Release = VersionBumpType.Patch},
        new ReleaseRule {Type = "perf", Release = VersionBumpType.Patch},
        new ReleaseRule {Type = "revert", Release = VersionBumpType.Patch},
        new ReleaseRule {Type = "docs", Release = VersionBumpType.None},
        new ReleaseRule {Type = "style", Release = VersionBumpType.None},
        new ReleaseRule {Type = "chore", Release = VersionBumpType.None},
        new ReleaseRule {Type = "refactor", Release = VersionBumpType.None},
        new ReleaseRule {Type = "test", Release = VersionBumpType.None},
        new ReleaseRule {Type = "build", Release = VersionBumpType.None},
        new ReleaseRule {Type = "ci", Release = VersionBumpType.None}
      ],
      BreakingChangesAlwaysMajor = true
    };
  }
}
