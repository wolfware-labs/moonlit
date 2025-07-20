namespace Wolfware.Moonlit.Plugins.SemanticRelease.Configuration;

public sealed class OpenAiClientConfiguration
{
  public string Model { get; set; } = "gpt-4o";

  public string ApiKey { get; set; } = string.Empty;
}
