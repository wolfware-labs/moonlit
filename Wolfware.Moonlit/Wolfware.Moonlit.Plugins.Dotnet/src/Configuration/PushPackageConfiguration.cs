namespace Wolfware.Moonlit.Plugins.Dotnet.Configuration;

public sealed class PublishPackageConfiguration
{
  public string Package { get; set; } = string.Empty;

  public string? Source { get; set; }

  public string? ApiKey { get; set; }
}
