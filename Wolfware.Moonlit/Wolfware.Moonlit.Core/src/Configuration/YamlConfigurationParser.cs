using Wolfware.Moonlit.Core.Configuration.Abstractions;
using Wolfware.Moonlit.Core.Configuration.Converters;
using YamlDotNet.Core;
using YamlDotNet.Core.Events;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace Wolfware.Moonlit.Core.Configuration;

/// <summary>
/// Provides functionality to parse YAML formatted release configurations into <see cref="ReleaseConfiguration"/> objects.
/// </summary>
/// <remarks>
/// This class uses the <see cref="YamlDotNet"/> library for deserialization. It is configured to:
/// - Use camel case naming convention.
/// - Ignore unmatched properties.
/// - Use specific type converters for handling step and plugin configurations.
/// </remarks>
public sealed class YamlConfigurationParser : IReleaseConfigurationParser
{
  public ReleaseConfiguration Parse(string configuration)
  {
    var deserializer = new DeserializerBuilder()
      .WithNamingConvention(CamelCaseNamingConvention.Instance)
      .IgnoreUnmatchedProperties()
      .WithTypeConverter(new StepConfigurationConverter())
      .WithTypeConverter(new PluginConfigurationConverter())
      .Build();
    var releaseConfiguration = deserializer.Deserialize<ReleaseConfiguration>(configuration);
    if (releaseConfiguration == null)
    {
      throw new InvalidOperationException(
        "Failed to parse the release configuration. Ensure the YAML format is correct.");
    }

    return releaseConfiguration;
  }
}
