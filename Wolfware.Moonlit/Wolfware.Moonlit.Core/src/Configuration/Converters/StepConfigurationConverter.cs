using YamlDotNet.Core;
using YamlDotNet.Core.Events;
using YamlDotNet.Serialization;

namespace Wolfware.Moonlit.Core.Configuration.Converters;

/// <summary>
/// A YAML type converter specifically for handling serialization and deserialization of <c>StepConfiguration</c> objects.
/// </summary>
/// <remarks>
/// This converter is designed to integrate with the YamlDotNet library, enabling custom parsing
/// and emitting of YAML for <c>StepConfiguration</c> instances. It can serialize objects to YAML
/// and deserialize YAML into object instances.
/// </remarks>
public sealed class StepConfigurationConverter : IYamlTypeConverter
{
  public bool Accepts(Type type)
  {
    return type == typeof(StepConfiguration);
  }

  public object? ReadYaml(IParser parser, Type type, ObjectDeserializer rootDeserializer)
  {
    if (parser.Current is not MappingStart)
    {
      throw new YamlException($"Expected mapping start but found {parser.Current}");
    }

    parser.MoveNext();

    var stepConfig = new StepConfiguration();
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
        case "run":
          if (parser.Current is Scalar runScalar)
          {
            var runValue = runScalar.Value;
            if (!string.IsNullOrEmpty(runValue))
            {
              var parts = runValue.Split('.', 2);
              if (parts.Length == 2)
              {
                stepConfig.PluginName = parts[0];
                stepConfig.MiddlewareName = parts[1];
              }
              else
              {
                throw new YamlException($"Invalid run format: {runValue}. Expected format: 'plugin.middleware'");
              }
            }

            parser.MoveNext();
          }
          else
          {
            throw new YamlException($"Expected scalar value for run but found {parser.Current}");
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

    stepConfig.Configuration = configDict;
    parser.MoveNext(); // Skip MappingEnd

    return stepConfig;
  }

  public void WriteYaml(IEmitter emitter, object? value, Type type, ObjectSerializer serializer)
  {
    if (value is not StepConfiguration stepConfig)
    {
      throw new YamlException($"Expected StepConfiguration object but found {value?.GetType().Name ?? "null"}");
    }

    emitter.Emit(new MappingStart());

    // Write run property
    if (!string.IsNullOrEmpty(stepConfig.PluginName) && !string.IsNullOrEmpty(stepConfig.MiddlewareName))
    {
      emitter.Emit(new Scalar(null, null, "run", ScalarStyle.Plain, true, false));
      emitter.Emit(new Scalar(null, null, $"{stepConfig.PluginName}.{stepConfig.MiddlewareName}", ScalarStyle.Plain,
        true, false));
    }

    // Write config property
    if (stepConfig.Configuration.Count > 0)
    {
      emitter.Emit(new Scalar(null, null, "config", ScalarStyle.Plain, true, false));
      emitter.Emit(new MappingStart());

      foreach (var (configKey, configValue) in stepConfig.Configuration)
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
