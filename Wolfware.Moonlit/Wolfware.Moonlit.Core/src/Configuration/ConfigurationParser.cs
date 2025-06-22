namespace Wolfware.Moonlit.Core.Configuration;

public static class ConfigurationParser
{
  public static ReleaseConfiguration ParseReleaseConfiguration(string? configuration)
  {
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));
    return new ReleaseConfiguration();
  }
}
