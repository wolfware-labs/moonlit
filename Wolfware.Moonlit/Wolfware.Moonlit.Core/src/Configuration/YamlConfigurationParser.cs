using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration.Converters;
using YamlDotNet.Core;
using YamlDotNet.Core.Events;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace Wolfware.Moonlit.Core.Configuration;

public class YamlConfigurationParser : IReleaseConfigurationParser
{
  public Task<ReleaseConfiguration> Parse(string configuration, CancellationToken cancellationToken = default)
  {
    var deserializer = new DeserializerBuilder()
      .WithNamingConvention(CamelCaseNamingConvention.Instance)
      .IgnoreUnmatchedProperties()
      .WithTypeConverter(new MiddlewareConfigurationConverter())
      .WithTypeConverter(new PluginConfigurationConverter())
      .Build();
    var releaseConfiguration = deserializer.Deserialize<ReleaseConfiguration>(configuration);
    if (releaseConfiguration == null)
    {
      throw new InvalidOperationException(
        "Failed to parse the release configuration. Ensure the YAML format is correct.");
    }

    return Task.FromResult(releaseConfiguration);
  }
}
