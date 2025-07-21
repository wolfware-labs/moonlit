using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class ChangelogGeneratorConfiguration
{
  public ChangelogRule[] Rules { get; set; } = [];

  public static ChangelogGeneratorConfiguration CreateDefault()
  {
    return new ChangelogGeneratorConfiguration
    {
      Rules =
      [
        new ChangelogRule {Type = "feat", Section = "Features", Icon = ":sparkles:", Summary = "New features"},
        new ChangelogRule {Type = "fix", Section = "Bug Fixes", Icon = ":bug:", Summary = "Bug fixes"},
        new ChangelogRule
        {
          Type = "perf", Section = "Performance Improvements", Icon = ":zap:", Summary = "Performance improvements"
        },
        new ChangelogRule
        {
          Type = "refactor", Section = "Code Refactoring", Icon = ":art:", Summary = "Code refactoring"
        },
        new ChangelogRule
        {
          Type = "style",
          Section = "Code Style Changes",
          Icon = ":lipstick:",
          Summary = "Code style changes (formatting, missing semi-colons, etc.)"
        },
        new ChangelogRule
        {
          Type = "test",
          Section = "Tests",
          Icon = ":white_check_mark:",
          Summary = "Adding missing tests or correcting existing tests"
        },
        new ChangelogRule
        {
          Type = "chore",
          Section = "Chores",
          Icon = ":wrench:",
          Summary = "Other changes that don't modify src or test files"
        },
        new ChangelogRule
        {
          Type = "docs", Section = "Documentation", Icon = ":book:", Summary = "Documentation only changes"
        },
        new ChangelogRule
        {
          Type = "build",
          Section = "Build System",
          Icon = ":construction_worker:",
          Summary = "Changes that affect the build system or external dependencies"
        },
        new ChangelogRule
        {
          Type = "ci",
          Section = "Continuous Integration",
          Icon = ":green_heart:",
          Summary = "Changes to our CI configuration files and scripts"
        },
        new ChangelogRule
        {
          Type = "revert", Section = "Reverts", Icon = ":rewind:", Summary = "Reverts a previous commit"
        },
        new ChangelogRule
        {
          Type = "breaking",
          Section = "Breaking Changes",
          IsBreakingChange = true,
          Icon = ":boom:",
          Summary = "Breaking changes"
        },
        new ChangelogRule
        {
          Type = "unknown",
          Section = "Other Changes",
          Icon = ":package:",
          Summary = "Other changes that don't fit into the above categories"
        }
      ]
    };
  }
}
