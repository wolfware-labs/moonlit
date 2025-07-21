using System.Dynamic;
using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Core.Extensions;

public static class ConfigurationSectionExtensions
{
  public static object? AsObject(this IConfigurationSection configurationSection)
  {
    return ConfigurationSectionExtensions.BuildConfigurationObject(configurationSection);
  }

  public static dynamic? AsDynamicObject(this IConfigurationSection configurationSection)
  {
    return ConfigurationSectionExtensions.BuildDynamicObject(configurationSection);
  }

  private static dynamic? BuildDynamicObject(IConfigurationSection section)
  {
    var children = section.GetChildren();

    if (!children.Any())
    {
      return section.Value?.ToClrType();
    }

    var isArray = children.All(child => int.TryParse(child.Key, out _));

    if (isArray)
    {
      return children.Select(ConfigurationSectionExtensions.BuildDynamicObject).ToArray();
    }

    IDictionary<string, object?> expando = new ExpandoObject();

    foreach (var child in children)
    {
      expando[child.Key] = ConfigurationSectionExtensions.BuildDynamicObject(child);
    }

    return expando;
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
