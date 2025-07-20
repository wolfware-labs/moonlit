using Wolfware.Moonlit.Plugins.SemanticRelease.Abstractions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public sealed class ChangelogGenerator : IChangelogGenerator
{
  public ChangelogCategory[] GenerateChangelog(ConventionalCommit[] commits,
    ChangelogGeneratorConfiguration configuration)
  {
    var categories = new List<ChangelogCategory>();

    foreach (var rule in configuration.Rules)
    {
      var matchingCommits = commits.Where(c => rule.Matches(c)).ToArray();
      if (matchingCommits.Length > 0)
      {
        categories.Add(new ChangelogCategory
        {
          Name = rule.Section,
          Icon = rule.Icon,
          Summary = rule.Summary,
          Entries = matchingCommits.Select(ChangelogGenerator.GetEntryFromCommit).ToArray()
        });
      }
    }

    return categories.ToArray();
  }

  private static ChangelogEntry GetEntryFromCommit(ConventionalCommit commit)
  {
    var description = string.IsNullOrWhiteSpace(commit.Scope)
      ? commit.Summary
      : $"**{commit.Scope}**: {commit.Summary}";

    return new ChangelogEntry {Sha = commit.Sha, Description = description};
  }
}
