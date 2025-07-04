using Semver;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease;

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
    var commits = commitMessages.Select(ConventionalCommitParser.Parse).ToList();
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

  public VersionBumpInfo AnalyzeCommits(SemVersion baseVersion, IEnumerable<string> commitMessages)
  {
    var commits = commitMessages.Select(SemanticRelease.ConventionalCommitParser.Parse).ToList();
    var nextVersion = CalculateNextVersion(baseVersion, commitMessages);
    var bumpType = SemanticVersionCalculator.GetBumpType(baseVersion, nextVersion);

    var commitAnalysis = commits.Select(commit => new CommitAnalysis
    {
      Commit = commit, BumpType = DetermineBumpType(commit), MatchedRule = FindMatchingRule(commit)
    }).ToList();

    return new VersionBumpInfo
    {
      BaseVersion = baseVersion,
      NextVersion = nextVersion,
      Commits = commits,
      CommitAnalysis = commitAnalysis,
      BumpType = bumpType,
      Configuration = _configuration
    };
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
    // Breaking changes always trigger major bump (if enabled)
    if (_configuration.BreakingChangesAlwaysMajor && commit.IsBreakingChange)
    {
      return VersionBumpType.Major;
    }

    // Find the first matching rule
    var matchingRule = FindMatchingRule(commit);
    return matchingRule?.Release ?? VersionBumpType.None;
  }

  private ReleaseRule? FindMatchingRule(ConventionalCommit commit)
  {
    return _configuration.Rules.FirstOrDefault(rule => rule.Matches(commit));
  }

  private static VersionBumpType GetBumpType(SemVersion baseVersion, SemVersion nextVersion)
  {
    if (nextVersion.Major > baseVersion.Major) return VersionBumpType.Major;
    if (nextVersion.Minor > baseVersion.Minor) return VersionBumpType.Minor;
    if (nextVersion.Patch > baseVersion.Patch) return VersionBumpType.Patch;
    return VersionBumpType.None;
  }
}
