namespace Wolfware.Moonlit.Plugins.Git.Configuration;

public sealed class CreateTagConfiguration
{
  public string Format { get; set; } = "{0}";

  public string Value { get; set; } = string.Empty;
}
