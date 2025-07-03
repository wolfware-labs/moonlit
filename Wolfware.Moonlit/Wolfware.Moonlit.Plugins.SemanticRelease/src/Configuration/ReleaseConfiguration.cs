using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class ReleaseConfiguration
{
  public List<ReleaseRule> Rules { get; set; } = [];

  public bool BreakingChangesAlwaysMajor { get; set; } = true;

  public static ReleaseConfiguration CreateDefault()
  {
    return new ReleaseConfiguration
    {
      Rules = new List<ReleaseRule>
      {
        new() {Type = "feat", Release = VersionBumpType.Minor},
        new() {Type = "fix", Release = VersionBumpType.Patch},
        new() {Type = "perf", Release = VersionBumpType.Patch},
        new() {Type = "revert", Release = VersionBumpType.Patch},
        new() {Type = "docs", Release = VersionBumpType.None},
        new() {Type = "style", Release = VersionBumpType.None},
        new() {Type = "chore", Release = VersionBumpType.None},
        new() {Type = "refactor", Release = VersionBumpType.None},
        new() {Type = "test", Release = VersionBumpType.None},
        new() {Type = "build", Release = VersionBumpType.None},
        new() {Type = "ci", Release = VersionBumpType.None}
      },
      BreakingChangesAlwaysMajor = true
    };
  }
}
