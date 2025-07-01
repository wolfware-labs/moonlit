using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Plugins.Extensions;

public static class ConfigurationExtensions
{
  public static T GetRequired<T>(this IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.Get<T>();
    if (config == null)
    {
      throw new InvalidOperationException($"Configuration for '{typeof(T).Name}' is not set.");
    }

    return config;
  }
}
