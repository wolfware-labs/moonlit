namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class GenerateChangelogsConfiguration
{
  public string[] Commits { get; set; } = [];

  public string OpenAiKey { get; set; } = string.Empty;
}
