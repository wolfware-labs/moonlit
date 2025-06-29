using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Configuration;

public sealed class ConfigurationFactory : IConfigurationFactory
{
  public IConfiguration Create(Dictionary<string, string?> configurationData)
  {
    var processedConfiguration = configurationData
      .ToDictionary(kvp => kvp.Key, kvp =>
      {
        if (kvp.Value is not { } strValue)
        {
          return kvp.Value;
        }

        var trimmedValue = strValue.Trim();
        return trimmedValue.StartsWith("@env.")
          ? Environment.GetEnvironmentVariable(trimmedValue[5..]) ?? string.Empty
          : trimmedValue;
      });

    return new ConfigurationBuilder()
      .AddInMemoryCollection(processedConfiguration)
      .Build();
  }
}
