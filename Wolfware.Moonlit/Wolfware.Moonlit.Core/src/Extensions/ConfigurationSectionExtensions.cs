using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Core.Extensions;

public static class ConfigurationSectionExtensions
{
  public static object? AsObject(this IConfigurationSection configurationSection)
  {
    return ConfigurationSectionExtensions.BuildConfigurationObject(configurationSection);
  }

  private static object? BuildConfigurationObject(IConfigurationSection section)
  {
    var children = section.GetChildren();

    if (!children.Any())
    {
      return section.Value;
    }

    var isArray = children.All(child => int.TryParse(child.Key, out _));

    if (isArray)
    {
      return children.Select(ConfigurationSectionExtensions.BuildConfigurationObject).ToArray();
    }

    return children.ToDictionary(child => child.Key, ConfigurationSectionExtensions.BuildConfigurationObject);
  }
}
