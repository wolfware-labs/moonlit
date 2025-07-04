using Semver;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public class SemanticVersionCalculator
{
  private readonly ReleaseConfiguration _configuration;

  public SemanticVersionCalculator(ReleaseConfiguration? configuration = null)
  {
    _configuration = configuration ?? ReleaseConfiguration.CreateDefault();
  }

  public SemVersion CalculateNextVersion(SemVersion baseVersion, IEnumerable<string> commitMessages,
    string? suffix = null)
  {
    var commits = commitMessages.Select(Services.ConventionalCommitParser.Parse).ToList();
    var highestBump = DetermineHighestBump(commits);

    var calculatedVersion = highestBump switch
    {
      VersionBumpType.Major => new SemVersion(baseVersion.Major + 1, 0, 0),
      VersionBumpType.Minor => new SemVersion(baseVersion.Major, baseVersion.Minor + 1, 0),
      VersionBumpType.Patch => new SemVersion(baseVersion.Major, baseVersion.Minor, baseVersion.Patch + 1),
      _ => baseVersion
    };

    if (!string.IsNullOrEmpty(suffix))
    {
      calculatedVersion = calculatedVersion.WithPrerelease(suffix, "1");
    }

    return calculatedVersion;
  }

  private VersionBumpType DetermineHighestBump(List<ConventionalCommit> commits)
  {
    var highestBump = VersionBumpType.None;

    foreach (var commit in commits)
    {
      var bumpType = DetermineBumpType(commit);
      if ((int)bumpType > (int)highestBump)
      {
        highestBump = bumpType;
      }
    }

    return highestBump;
  }

  private VersionBumpType DetermineBumpType(ConventionalCommit commit)
  {
    if (_configuration.BreakingChangesAlwaysMajor && commit.IsBreakingChange)
    {
      return VersionBumpType.Major;
    }

    var matchingRule = FindMatchingRule(commit);
    return matchingRule?.Release ?? VersionBumpType.None;
  }

  private ReleaseRule? FindMatchingRule(ConventionalCommit commit)
  {
    return _configuration.Rules.FirstOrDefault(rule => rule.Matches(commit));
  }
}
