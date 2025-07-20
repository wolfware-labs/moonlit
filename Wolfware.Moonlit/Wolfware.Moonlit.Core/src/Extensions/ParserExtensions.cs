/***********************************************
Author: Mariano Santoro
Description: The ParserExtensions
Created On: 07/20/2025
Modified By:
Modified On:
Modified Comments:
Ticket Number:
************************************************/

using YamlDotNet.Core;
using YamlDotNet.Core.Events;

namespace Wolfware.Moonlit.Core.Extensions;

public static class ParserExtensions
{
  public static Dictionary<string, object?> ParseMap(this IParser parser)
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
          map[key] = parser.ParseSequence();
          break;
        case MappingStart:
          map[key] = parser.ParseMap();
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

  public static List<object?> ParseSequence(this IParser parser)
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
          sequenceValues.Add(parser.ParseSequence());
          break;
        case MappingStart:
          sequenceValues.Add(parser.ParseMap());
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
