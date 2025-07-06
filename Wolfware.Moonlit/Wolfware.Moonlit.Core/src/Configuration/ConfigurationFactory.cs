using System.Text.Json;
using System.Text.RegularExpressions;
using Configuration.Extensions.EnvironmentFile;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Extensions;

namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Factory class responsible for creating instances of <see cref="IConfiguration"/> objects.
/// </summary>
/// <remarks>
/// Provides methods to create base configurations and build configurations using in-memory data
/// with optional support for parent configurations and dynamic replacement of configuration expressions.
/// </remarks>
public sealed class ConfigurationFactory : IConfigurationFactory
{
  private readonly IConfigurationExpressionParser _expressionParser;

  public ConfigurationFactory(IConfigurationExpressionParser expressionParser)
  {
    _expressionParser = expressionParser;
  }

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

    var processedConfiguration = new Dictionary<string, object?>();
    foreach ((var key, var value) in configurationData)
    {
      if (value is not string stringValue || string.IsNullOrWhiteSpace(stringValue))
      {
        processedConfiguration[key] = value;
        continue;
      }

      var parsedValue = this._expressionParser.ParseExpression(stringValue, parentConfiguration);
      if (parsedValue is null)
      {
        continue;
      }

      processedConfiguration[key] = parsedValue;
    }

    var jsonStream = new MemoryStream(JsonSerializer.SerializeToUtf8Bytes(processedConfiguration));

    return new ConfigurationBuilder()
      .AddConfiguration(parentConfiguration)
      .AddJsonStream(jsonStream)
      .Build();
  }
}
