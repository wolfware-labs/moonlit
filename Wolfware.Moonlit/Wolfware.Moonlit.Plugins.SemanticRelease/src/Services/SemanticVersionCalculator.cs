using Semver;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public sealed class SemanticVersionCalculator
{
  private readonly CommitsAnalyzer _commitsAnalyzer;

  public SemanticVersionCalculator(CommitsAnalyzer commitsAnalyzer)
  {
    _commitsAnalyzer = commitsAnalyzer;
  }

  public SemVersion CalculateNextVersion(SemVersion baseVersion, ConventionalCommit[] commits, string? suffix = null)
  {
    ArgumentNullException.ThrowIfNull(baseVersion, nameof(baseVersion));
    ArgumentNullException.ThrowIfNull(commits, nameof(commits));
    if (commits.Length == 0)
    {
      throw new ArgumentException("At least one commit is required to calculate the next version.", nameof(commits));
    }

    var highestBump = this._commitsAnalyzer.Analyze(commits);
    var calculatedVersion = highestBump switch
    {
      VersionBumpType.Major => baseVersion.WithMajor(baseVersion.Major + 1).WithMinor(0).WithPatch(0),
      VersionBumpType.Minor => baseVersion.WithMinor(baseVersion.Minor + 1).WithPatch(0),
      VersionBumpType.Patch => baseVersion.WithPatch(baseVersion.Patch + 1),
      _ => baseVersion
    };

    if (!string.IsNullOrEmpty(suffix))
    {
      calculatedVersion = calculatedVersion.WithPrerelease(suffix, "1");
    }

    return calculatedVersion;
  }
}
