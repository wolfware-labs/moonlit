using Wolfware.Moonlit.Core.Extensions;
using YamlDotNet.Core;
using YamlDotNet.Core.Events;
using YamlDotNet.Serialization;

namespace Wolfware.Moonlit.Core.Configuration.Converters;

/// <summary>
/// A custom YAML type converter for the `PluginConfiguration` class.
/// Converts instances of `PluginConfiguration` to and from YAML format.
/// </summary>
public sealed class PluginConfigurationConverter : IYamlTypeConverter
{
  public bool Accepts(Type type)
  {
    return type == typeof(PluginConfiguration);
  }

  public object? ReadYaml(IParser parser, Type type, ObjectDeserializer rootDeserializer)
  {
    if (parser.Current is not MappingStart)
    {
      throw new YamlException($"Expected mapping start but found {parser.Current}");
    }

    parser.MoveNext();

    var pluginConfig = new PluginConfiguration();
    var configDict = new Dictionary<string, object?>();

    while (parser.Current is not MappingEnd)
    {
      if (parser.Current is not Scalar keyScalar)
      {
        throw new YamlException($"Expected scalar key but found {parser.Current}");
      }

      var key = keyScalar.Value;
      parser.MoveNext();

      switch (key.ToLowerInvariant())
      {
        case "name":
          if (parser.Current is Scalar nameScalar)
          {
            pluginConfig.Name = nameScalar.Value;
            parser.MoveNext();
          }
          else
          {
            throw new YamlException($"Expected scalar value for name but found {parser.Current}");
          }

          break;

        case "url":
          if (parser.Current is Scalar urlScalar)
          {
            if (Uri.TryCreate(urlScalar.Value, UriKind.Absolute, out var uri))
            {
              pluginConfig.Url = uri;
            }
            else
            {
              throw new YamlException($"Invalid URI format: {urlScalar.Value}");
            }

            parser.MoveNext();
          }
          else
          {
            throw new YamlException($"Expected scalar value for url but found {parser.Current}");
          }

          break;

        case "config":
          configDict = parser.ParseMap();
          break;

        default:
          // Skip unknown properties
          parser.SkipThisAndNestedEvents();
          break;
      }
    }

    pluginConfig.Configuration = configDict;
    parser.MoveNext(); // Skip MappingEnd

    return pluginConfig;
  }

  public void WriteYaml(IEmitter emitter, object? value, Type type, ObjectSerializer serializer)
  {
    throw new NotSupportedException("Writing YAML for PluginConfiguration is not supported.");
  }
}
