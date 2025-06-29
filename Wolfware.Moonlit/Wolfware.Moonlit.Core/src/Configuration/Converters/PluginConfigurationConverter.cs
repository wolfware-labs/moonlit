using YamlDotNet.Core;
using YamlDotNet.Core.Events;
using YamlDotNet.Serialization;

namespace Wolfware.Moonlit.Core.Configuration.Converters;

/// <summary>
/// A custom YAML type converter for the `PluginConfiguration` class.
/// Converts instances of `PluginConfiguration` to and from YAML format.
/// </summary>
public class PluginConfigurationConverter : IYamlTypeConverter
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
    var configDict = new Dictionary<string, string?>();

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
          if (parser.Current is MappingStart)
          {
            parser.MoveNext();

            while (parser.Current is not MappingEnd)
            {
              if (parser.Current is not Scalar configKeyScalar)
              {
                throw new YamlException($"Expected scalar key in config but found {parser.Current}");
              }

              var configKey = configKeyScalar.Value;
              parser.MoveNext();

              // Handle null values or scalar values
              if (parser.Current is Scalar configValueScalar)
              {
                configDict[configKey] = configValueScalar.Value;
                parser.MoveNext();
              }
              else if (parser.Current is SequenceStart)
              {
                // Skip arrays and complex structures, we only support string values
                parser.SkipThisAndNestedEvents();
              }
              else if (parser.Current is MappingStart)
              {
                // Skip nested objects, we only support string values
                parser.SkipThisAndNestedEvents();
              }
              else
              {
                // Handle null or other scalar types
                configDict[configKey] = null;
                parser.MoveNext();
              }
            }

            parser.MoveNext(); // Skip MappingEnd
          }
          else
          {
            // Skip non-mapping config values
            parser.SkipThisAndNestedEvents();
          }

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
    if (value is not PluginConfiguration pluginConfig)
    {
      throw new YamlException($"Expected PluginConfiguration object but found {value?.GetType().Name ?? "null"}");
    }

    emitter.Emit(new MappingStart());

    // Emit name
    emitter.Emit(new Scalar(null, null, "name", ScalarStyle.DoubleQuoted, true, false));
    emitter.Emit(new Scalar(null, null, pluginConfig.Name, ScalarStyle.DoubleQuoted, true, false));

    // Emit url
    emitter.Emit(new Scalar(null, null, "url", ScalarStyle.Plain, true, false));
    emitter.Emit(new Scalar(null, null, pluginConfig.Url.ToString(), ScalarStyle.DoubleQuoted, true, false));

    // Emit configuration if present
    if (pluginConfig.Configuration.Count > 0)
    {
      emitter.Emit(new Scalar(null, null, "config", ScalarStyle.Plain, true, false));
      emitter.Emit(new MappingStart());

      foreach (var (configKey, configValue) in pluginConfig.Configuration)
      {
        emitter.Emit(new Scalar(null, null, configKey, ScalarStyle.Plain, true, false));

        if (configValue is null)
        {
          // Emit null value
          emitter.Emit(new Scalar(null, null, string.Empty, ScalarStyle.Plain, false, false));
        }
        else
        {
          // For environment variables or other values that might need to be preserved as-is
          bool needsQuoting = configValue.Contains('$') || configValue.Contains(' ') ||
                              string.IsNullOrEmpty(configValue);
          var style = needsQuoting ? ScalarStyle.DoubleQuoted : ScalarStyle.Plain;
          emitter.Emit(new Scalar(null, null, configValue, style, true, false));
        }
      }

      emitter.Emit(new MappingEnd());
    }

    emitter.Emit(new MappingEnd());
  }
}
