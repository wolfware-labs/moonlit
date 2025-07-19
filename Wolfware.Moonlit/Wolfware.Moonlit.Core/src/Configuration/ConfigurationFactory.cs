using System.Collections;
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
      .AddEnvironmentVariables("MOONLIT_")
      .AddEnvironmentFile()
      .Build();
  }

  /// <inheritdoc />
  public IConfiguration Create(Dictionary<string, object?> configurationData,
    IConfiguration? parentConfiguration = null)
  {
    parentConfiguration ??= this.CreateBaseConfiguration();

    var processedConfiguration = this.ProcessDictionary(configurationData, parentConfiguration);
    var jsonStream = new MemoryStream(JsonSerializer.SerializeToUtf8Bytes(processedConfiguration));

    return new ConfigurationBuilder()
      .AddConfiguration(parentConfiguration)
      .AddJsonStream(jsonStream)
      .Build();
  }

  private Dictionary<string, object?> ProcessDictionary(Dictionary<string, object?> dictionary,
    IConfiguration context)
  {
    var processedConfiguration = new Dictionary<string, object?>();
    foreach ((var key, var value) in dictionary)
    {
      if (value is null)
      {
        processedConfiguration[key] = null;
        continue;
      }

      if (value is string stringValue)
      {
        var parsedValue = this._expressionParser.ParseExpression(stringValue, context);
        processedConfiguration[key] = parsedValue;
        continue;
      }

      if (value is IDictionary<string, object?> dictValue)
      {
        var processedDict = this.ProcessDictionary(dictValue.ToDictionary(kv => kv.Key, kv => kv.Value), context);
        processedConfiguration[key] = processedDict;
        continue;
      }

      if (value is IEnumerable enumValue)
      {
        var processedEnum = this.ProcessEnumerable(enumValue, context);
        processedConfiguration[key] = processedEnum;
        continue;
      }

      processedConfiguration[key] = value;
    }

    return processedConfiguration;
  }

  private object?[] ProcessEnumerable(IEnumerable enumerable, IConfiguration context)
  {
    var processedList = new List<object?>();
    foreach (var item in enumerable)
    {
      if (item is null)
      {
        processedList.Add(null);
        continue;
      }

      if (item is string stringValue)
      {
        var parsedValue = this._expressionParser.ParseExpression(stringValue, context);
        processedList.Add(parsedValue);
        continue;
      }

      if (item is IDictionary<string, object?> dictItem)
      {
        var processedDict = this.ProcessDictionary(dictItem.ToDictionary(kv => kv.Key, kv => kv.Value), context);
        processedList.Add(processedDict);
        continue;
      }

      if (item is IEnumerable enumItem)
      {
        var processedEnum = this.ProcessEnumerable(enumItem, context);
        processedList.Add(processedEnum);
        continue;
      }

      processedList.Add(item);
    }

    return processedList.ToArray();
  }
}
