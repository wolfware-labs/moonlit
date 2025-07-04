using System.Text.RegularExpressions;
using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Services;

public partial class ConventionalCommitParser
{
  private static readonly Regex _commitRegex = ConventionalCommitParser.GetCommitRegex();

  public static ConventionalCommit Parse(string commitMessage)
  {
    var match = ConventionalCommitParser._commitRegex.Match(commitMessage.Split('\n')[0].Trim());

    if (!match.Success)
    {
      return new ConventionalCommit {Type = "unknown", Description = commitMessage, IsBreakingChange = false};
    }

    var hasBreakingFooter = commitMessage.Contains("BREAKING CHANGE:", StringComparison.OrdinalIgnoreCase);

    return new ConventionalCommit
    {
      Type = match.Groups["type"].Value.ToLowerInvariant(),
      Scope = match.Groups["scope"].Value,
      Description = match.Groups["description"].Value,
      IsBreakingChange = match.Groups["breaking"].Success || hasBreakingFooter,
      FullMessage = commitMessage
    };
  }

  [GeneratedRegex(@"^(?<type>\w+)(?:\((?<scope>[\w\-_]+)\))?(?<breaking>!)?:\s*(?<description>.*)$",
    RegexOptions.Multiline | RegexOptions.Compiled)]
  private static partial Regex GetCommitRegex();
}
