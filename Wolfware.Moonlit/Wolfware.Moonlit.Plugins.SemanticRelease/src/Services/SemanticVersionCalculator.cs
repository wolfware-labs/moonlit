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

    var bumpType = _commitsAnalyzer.Analyze(commits);

    if (string.IsNullOrWhiteSpace(suffix))
    {
      return string.IsNullOrEmpty(baseVersion.Prerelease)
        ? SemanticVersionCalculator.GetBumpedVersion(baseVersion, bumpType)
        : baseVersion.WithoutPrerelease();
    }

    (var currentLabel, var currentIteration) = SemanticVersionCalculator.GetPreReleaseInfo(baseVersion);
    var currentVersionLevel = SemanticVersionCalculator.GetVersionLevel(baseVersion);

    if (currentLabel == suffix && bumpType <= currentVersionLevel)
    {
      return new SemVersion(
        baseVersion.Major,
        baseVersion.Minor,
        baseVersion.Patch
      ).WithPrerelease(
        currentLabel,
        (currentIteration + 1).ToString()
      );
    }

    return SemanticVersionCalculator.GetBumpedVersion(baseVersion, bumpType).WithPrerelease(suffix, "1");
  }

  private static SemVersion GetBumpedVersion(SemVersion version, VersionBumpType bumpType)
  {
    return bumpType switch
    {
      VersionBumpType.Major => new SemVersion(version.Major + 1, 0, 0),
      VersionBumpType.Minor => new SemVersion(version.Major, version.Minor + 1, 0),
      VersionBumpType.Patch => new SemVersion(version.Major, version.Minor, version.Patch + 1),
      _ => throw new ArgumentOutOfRangeException(nameof(bumpType))
    };
  }

  private static VersionBumpType GetVersionLevel(SemVersion version)
  {
    if (version.Major > 0 && version.Minor == 0 && version.Patch == 0)
    {
      return VersionBumpType.Major;
    }

    if (version.Minor > 0 && version.Patch == 0)
    {
      return VersionBumpType.Minor;
    }

    return VersionBumpType.Patch;
  }

  private static (string Label, int Iteration) GetPreReleaseInfo(SemVersion version)
  {
    if (string.IsNullOrEmpty(version.Prerelease) || version.PrereleaseIdentifiers.Count < 2)
    {
      return (string.Empty, 0);
    }

    return (version.PrereleaseIdentifiers[0], int.Parse(version.PrereleaseIdentifiers[1]));
  }
}
