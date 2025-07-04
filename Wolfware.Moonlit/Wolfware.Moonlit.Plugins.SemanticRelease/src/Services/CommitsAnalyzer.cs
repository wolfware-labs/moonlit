using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public sealed class CommitsAnalyzer
{
  private readonly CommitsAnalyzerConfiguration _configuration;

  public CommitsAnalyzer(CommitsAnalyzerConfiguration configuration)
  {
    _configuration = configuration;
  }

  public VersionBumpType Analyze(ConventionalCommit[] commits)
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
