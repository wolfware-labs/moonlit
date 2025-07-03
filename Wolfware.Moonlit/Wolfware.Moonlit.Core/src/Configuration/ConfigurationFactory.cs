using System.Text.Json;
using System.Text.RegularExpressions;
using Configuration.Extensions.EnvironmentFile;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Factory class responsible for creating instances of <see cref="IConfiguration"/> objects.
/// </summary>
/// <remarks>
/// Provides methods to create base configurations and build configurations using in-memory data
/// with optional support for parent configurations and dynamic replacement of configuration expressions.
/// </remarks>
public sealed partial class ConfigurationFactory : IConfigurationFactory
{
  /// <inheritdoc />
  public IConfiguration CreateBaseConfiguration()
  {
    return new ConfigurationBuilder()
      .AddEnvironmentFile()
      .AddEnvironmentVariables("MOONLIT_")
      .Build();
  }

  /// <inheritdoc />
  public IConfiguration Create(Dictionary<string, object?> configurationData,
    IConfiguration? parentConfiguration = null)
  {
    parentConfiguration ??= this.CreateBaseConfiguration();

    var regex = ConfigurationFactory.ConfigExpressionRegex();
    var processedConfiguration = configurationData
      .ToDictionary(kvp => kvp.Key, kvp =>
      {
        if (kvp.Value is not string valueString || string.IsNullOrWhiteSpace(valueString))
        {
          return kvp.Value;
        }

        var value = valueString.Trim();
        var match = regex.Match(value);
        if (!match.Success || !match.Groups.TryGetValue("config_expression", out var configExpression))
        {
          return value;
        }

        var configSection = parentConfiguration.GetSection(configExpression.Value);
        if (!configSection.Exists())
        {
          return kvp.Value;
        }

        return configSection.Value != null
          ? configSection.Get<object>()
          : configSection.Get<Dictionary<string, object>>();
      });

    var jsonStream = new MemoryStream(JsonSerializer.SerializeToUtf8Bytes(processedConfiguration));

    return new ConfigurationBuilder()
      .AddConfiguration(parentConfiguration)
      .AddJsonStream(jsonStream)
      .Build();
  }

  [GeneratedRegex(@"\$\((?<config_expression>[^\)]+)\)")]
  private static partial Regex ConfigExpressionRegex();
}
