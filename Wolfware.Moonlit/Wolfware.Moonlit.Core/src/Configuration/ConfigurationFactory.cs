using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Responsible for creating and configuring an instance of <see cref="IConfiguration"/>
/// based on the provided dictionary of configuration data.
/// </summary>
/// <remarks>
/// This class processes the configuration data dictionary by trimming values and resolving environment variable references.
/// If a value starts with "@env.", it attempts to retrieve the corresponding environment variable.
/// </remarks>
public sealed class ConfigurationFactory : IConfigurationFactory
{
  /// <inheritdoc />
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
