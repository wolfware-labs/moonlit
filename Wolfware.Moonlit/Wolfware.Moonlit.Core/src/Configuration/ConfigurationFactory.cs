using System.Text.RegularExpressions;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Configuration;

public sealed partial class ConfigurationFactory : IConfigurationFactory
{
  /// <inheritdoc />
  public IConfiguration Create(Dictionary<string, string?> configurationData,
    IConfiguration? parentConfiguration = null)
  {
    parentConfiguration ??= new ConfigurationBuilder().AddEnvironmentVariables().Build();

    var regex = ConfigurationFactory.ConfigExpressionRegex();
    var processedConfiguration = configurationData
      .ToDictionary(kvp => kvp.Key, kvp =>
      {
        if (string.IsNullOrWhiteSpace(kvp.Value))
        {
          return null;
        }

        var value = kvp.Value.Trim();
        var match = regex.Match(value);
        if (match.Success && match.Groups.TryGetValue("config_expression", out var configExpression))
        {
          return parentConfiguration[configExpression.Value];
        }

        return value;
      });

    return new ConfigurationBuilder()
      .AddConfiguration(parentConfiguration)
      .AddInMemoryCollection(processedConfiguration)
      .Build();
  }

  [GeneratedRegex(@"\$\((?<config_expression>[^\)]+)\)")]
  private static partial Regex ConfigExpressionRegex();
}
