using Semver;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Models;

public class VersionBumpInfo
{
  public SemVersion BaseVersion { get; set; } = new(0, 0, 0);

  public SemVersion NextVersion { get; set; } = new(0, 0, 0);

  public List<ConventionalCommit> Commits { get; set; } = [];

  public List<CommitAnalysis> CommitAnalysis { get; set; } = [];

  public VersionBumpType BumpType { get; set; }

  public ReleaseConfiguration Configuration { get; set; } = ReleaseConfiguration.CreateDefault();

  public void PrintSummary()
  {
    Console.WriteLine($"Base Version: {BaseVersion}");
    Console.WriteLine($"Next Version: {NextVersion}");
    Console.WriteLine($"Bump Type: {BumpType}");
    Console.WriteLine();

    var breakingChanges = CommitAnalysis.Where(c => c.Commit.IsBreakingChange).ToList();
    var features = CommitAnalysis.Where(c => c is {BumpType: VersionBumpType.Minor, Commit.IsBreakingChange: false})
      .ToList();
    var fixes = CommitAnalysis.Where(c => c.BumpType == VersionBumpType.Patch).ToList();
    var others = CommitAnalysis.Where(c => c is {BumpType: VersionBumpType.None, Commit.IsBreakingChange: false}).ToList();

    if (breakingChanges.Count != 0)
    {
      Console.WriteLine("BREAKING CHANGES:");
      foreach (var analysis in breakingChanges)
      {
        Console.WriteLine($"  - {analysis.Commit.Type}: {analysis.Commit.Description}");
      }

      Console.WriteLine();
    }

    if (features.Count != 0)
    {
      Console.WriteLine("Features:");
      foreach (var analysis in features)
      {
        Console.WriteLine($"  - {analysis.Commit.Description}");
      }

      Console.WriteLine();
    }

    if (fixes.Count != 0)
    {
      Console.WriteLine("Bug Fixes:");
      foreach (var analysis in fixes)
      {
        Console.WriteLine($"  - {analysis.Commit.Description}");
      }

      Console.WriteLine();
    }

    if (others.Count != 0)
    {
      Console.WriteLine("Other Changes:");
      foreach (var analysis in others)
      {
        Console.WriteLine($"  - {analysis.Commit.Type}: {analysis.Commit.Description}");
      }
    }
  }

  public void PrintDetailedAnalysis()
  {
    Console.WriteLine("=== Detailed Commit Analysis ===");
    foreach (var analysis in CommitAnalysis)
    {
      var ruleInfo = analysis.MatchedRule != null
        ? $"Rule: {analysis.MatchedRule.Type ?? "custom"}"
        : "No matching rule";

      Console.WriteLine($"'{analysis.Commit.FullMessage}'");
      Console.WriteLine($"  Type: {analysis.Commit.Type}, Scope: {analysis.Commit.Scope ?? "none"}");
      Console.WriteLine($"  Breaking: {analysis.Commit.IsBreakingChange}, Bump: {analysis.BumpType}");
      Console.WriteLine($"  {ruleInfo}");
      Console.WriteLine();
    }
  }
}
