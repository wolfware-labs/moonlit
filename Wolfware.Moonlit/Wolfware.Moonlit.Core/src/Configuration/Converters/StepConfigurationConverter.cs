using Wolfware.Moonlit.Core.Extensions;
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
            stepConfig.StepName = nameScalar.Value;
            parser.MoveNext();
          }
          else
          {
            throw new YamlException($"Expected scalar value for name but found {parser.Current}");
          }

          break;
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
          configDict = parser.ParseMap();
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
    throw new NotSupportedException("Writing YAML for StepConfiguration is not supported.");
  }
}
