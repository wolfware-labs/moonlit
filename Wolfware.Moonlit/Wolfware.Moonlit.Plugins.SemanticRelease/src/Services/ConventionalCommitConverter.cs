using System.Text.RegularExpressions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public partial class ConventionalCommitConverter
{
  private static readonly Regex _commitRegex = ConventionalCommitConverter.GetCommitRegex();

  public static ConventionalCommit[] Convert(params Commit[] commits)
  {
    return commits.Select(ConventionalCommitConverter.ConvertCommit).ToArray();
  }

  private static ConventionalCommit ConvertCommit(Commit commit)
  {
    var match = ConventionalCommitConverter._commitRegex.Match(commit.Message.Split('\n')[0].Trim());

    if (!match.Success)
    {
      return new ConventionalCommit
      {
        Type = "unknown",
        Description = commit.Message,
        IsBreakingChange = false,
        FullMessage = commit.Message,
        Date = commit.Date,
        Sha = commit.Sha[..7]
      };
    }

    var hasBreakingFooter = commit.Message.Contains("BREAKING CHANGE:", StringComparison.OrdinalIgnoreCase);

    return new ConventionalCommit
    {
      Type = match.Groups["type"].Value.ToLowerInvariant(),
      Scope = match.Groups["scope"].Value,
      Description = match.Groups["description"].Value,
      IsBreakingChange = match.Groups["breaking"].Success || hasBreakingFooter,
      FullMessage = commit.Message,
      Date = commit.Date,
      Sha = commit.Sha[..7]
    };
  }

  [GeneratedRegex(@"^(?<type>\w+)(?:\((?<scope>[\w\-_]+)\))?(?<breaking>!)?:\s*(?<description>.*)$",
    RegexOptions.Multiline | RegexOptions.Compiled)]
  private static partial Regex GetCommitRegex();
}
