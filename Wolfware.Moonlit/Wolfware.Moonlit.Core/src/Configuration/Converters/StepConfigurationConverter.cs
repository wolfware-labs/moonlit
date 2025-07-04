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
          configDict = this.ParseMap(parser);
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

  private Dictionary<string, object?> ParseMap(IParser parser)
  {
    var map = new Dictionary<string, object?>();
    if (parser.Current is not MappingStart)
    {
      throw new YamlException($"Expected mapping start for configuration but found {parser.Current}");
    }

    parser.MoveNext();
    while (parser.Current is not MappingEnd)
    {
      if (parser.Current is not Scalar keyScalar)
      {
        throw new YamlException($"Expected scalar key in configuration but found {parser.Current}");
      }

      var key = keyScalar.Value;
      parser.MoveNext();

      switch (parser.Current)
      {
        case Scalar valueScalar:
          map[key] = valueScalar.Value;
          parser.MoveNext();
          break;
        case SequenceStart:
          map[key] = this.ParseSequence(parser);
          break;
        case MappingStart:
          map[key] = this.ParseMap(parser);
          break;
        default:
          map[key] = null;
          parser.MoveNext();
          break;
      }
    }

    parser.MoveNext(); // Skip MappingEnd
    return map;
  }

  private List<object?> ParseSequence(IParser parser)
  {
    var sequenceValues = new List<object?>();
    if (parser.Current is not SequenceStart)
    {
      throw new YamlException($"Expected sequence start but found {parser.Current}");
    }

    parser.MoveNext(); // Move to first item in sequence

    while (parser.Current is not SequenceEnd)
    {
      switch (parser.Current)
      {
        case Scalar scalar:
          sequenceValues.Add(scalar.Value);
          break;
        case SequenceStart:
          sequenceValues.Add(this.ParseSequence(parser));
          break;
        case MappingStart:
          sequenceValues.Add(this.ParseMap(parser));
          break;
        default:
          // Handle null or other scalar types
          sequenceValues.Add(null);
          parser.MoveNext();
          break;
      }

      parser.MoveNext(); // Move to next item in sequence
    }

    parser.MoveNext(); // Skip SequenceEnd
    return sequenceValues;
  }
}
