using Wolfware.Moonlit.Plugins.SemanticRelease.Models;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class GenerateChangelogConfiguration
{
  public CommitMessage[] Commits { get; set; } = [];

  public string? OpenAiKey { get; set; } = string.Empty;
}
