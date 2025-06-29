using YamlDotNet.Core;
using YamlDotNet.Core.Events;
using YamlDotNet.Serialization;

namespace Wolfware.Moonlit.Core.Configuration.Converters;

public sealed class MiddlewareConfigurationConverter : IYamlTypeConverter
{
  public bool Accepts(Type type)
  {
    return type == typeof(MiddlewareConfiguration);
  }

  public object? ReadYaml(IParser parser, Type type, ObjectDeserializer rootDeserializer)
  {
    var middleware = new MiddlewareConfiguration();
    var configDict = new Dictionary<string, string>();

    // Check if we're looking at a mapping
    if (parser.Current is MappingStart)
    {
      parser.MoveNext();

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
              var fullName = nameScalar.Value;
              var parts = fullName.Split('.');

              if (parts.Length == 2)
              {
                middleware.Plugin = parts[0];
                middleware.Name = parts[1];
              }
              else
              {
                // If the format is not plugin.name, use the entire string as the name
                middleware.Name = fullName;
              }

              parser.MoveNext();
            }
            else
            {
              throw new YamlException($"Expected scalar value for name but found {parser.Current}");
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

                if (parser.Current is Scalar configValueScalar)
                {
                  configDict[configKey] = configValueScalar.Value;
                  parser.MoveNext();
                }
                else
                {
                  // Skip non-scalar values in config
                  parser.SkipThisAndNestedEvents();
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

      parser.MoveNext(); // Skip MappingEnd
    }
    else if (parser.Current is Scalar scalar)
    {
      // Handle the case where the middleware is specified as just a string
      var fullName = scalar.Value;
      var parts = fullName.Split('.');

      if (parts.Length == 2)
      {
        middleware.Plugin = parts[0];
        middleware.Name = parts[1];
      }
      else
      {
        // If the format is not plugin.name, use the entire string as the name
        middleware.Name = fullName;
      }

      parser.MoveNext();
    }
    else
    {
      throw new YamlException($"Expected mapping or scalar but found {parser.Current}");
    }

    middleware.Configuration = configDict;
    return middleware;
  }

  public void WriteYaml(IEmitter emitter, object? value, Type type, ObjectSerializer serializer)
  {
    if (value is not MiddlewareConfiguration middleware)
    {
      throw new YamlException($"Expected MiddlewareConfiguration object but found {value?.GetType().Name ?? "null"}");
    }

    // Check if this is a simple middleware or one with configuration
    if (middleware.Configuration.Count == 0)
    {
      // Simple middleware can be written as a scalar with plugin.name format
      var fullName = string.IsNullOrEmpty(middleware.Plugin)
        ? middleware.Name
        : $"{middleware.Plugin}.{middleware.Name}";

      emitter.Emit(new Scalar(null, null, fullName, ScalarStyle.Plain, true, false));
    }
    else
    {
      // Complex middleware needs to be written as a mapping
      emitter.Emit(new MappingStart());

      // Write the name as plugin.name
      var fullName = string.IsNullOrEmpty(middleware.Plugin)
        ? middleware.Name
        : $"{middleware.Plugin}.{middleware.Name}";

      emitter.Emit(new Scalar(null, null, "name", ScalarStyle.Plain, true, false));
      emitter.Emit(new Scalar(null, null, fullName, ScalarStyle.Plain, true, false));

      // Write the configuration if any exists
      if (middleware.Configuration.Count > 0)
      {
        emitter.Emit(new Scalar(null, null, "config", ScalarStyle.Plain, true, false));
        emitter.Emit(new MappingStart());

        foreach ((var configKey, var configValue) in middleware.Configuration)
        {
          emitter.Emit(new Scalar(null, null, configKey, ScalarStyle.Plain, true, false));
          emitter.Emit(new Scalar(null, null, configValue, ScalarStyle.Plain, true, false));
        }

        emitter.Emit(new MappingEnd());
      }

      emitter.Emit(new MappingEnd());
    }
  }
}
