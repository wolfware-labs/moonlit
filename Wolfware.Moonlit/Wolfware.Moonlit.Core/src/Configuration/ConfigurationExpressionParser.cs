using System.Text.RegularExpressions;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Extensions;

namespace Wolfware.Moonlit.Core.Configuration;

public partial class ConfigurationExpressionParser : IConfigurationExpressionParser
{
  private static readonly Regex _embeddedConfigExpressionRegex =
    ConfigurationExpressionParser.GetEmbeddedConfigExpressionRegex();

  private static readonly Regex _configExpressionRegex = ConfigurationExpressionParser.GetConfigExpressionRegex();

  public object? ParseExpression(string expression, IConfiguration context)
  {
    ArgumentNullException.ThrowIfNull(expression);
    ArgumentNullException.ThrowIfNull(context);

    if (string.IsNullOrWhiteSpace(expression))
    {
      return null;
    }

    var extractionResult = ConfigurationExpressionParser.ExtractExpression(expression);
    if (extractionResult is null)
    {
      return expression;
    }

    (var embedded, var configExpression) = extractionResult.Value;
    var configSection = context.GetSection(configExpression);
    if (!configSection.Exists())
    {
      return null;
    }

    var configValue = configSection.AsObject();
    return embedded ? ConfigurationExpressionParser.Replace(expression, configValue?.ToString()) : configValue;
  }

  private static (bool Embedded, string configExpression)? ExtractExpression(string expression)
  {
    var match = _configExpressionRegex.Match(expression);
    if (match.Success && match.Groups.TryGetValue("config_expression", out var configExpression))
    {
      return (false, configExpression.Value);
    }

    match = _embeddedConfigExpressionRegex.Match(expression);
    if (match.Success && match.Groups.TryGetValue("config_expression", out configExpression))
    {
      return (true, configExpression.Value);
    }

    return null;
  }

  private static string Replace(string expression, string? value)
  {
    return ConfigurationExpressionParser._embeddedConfigExpressionRegex.Replace(expression, value ?? string.Empty);
  }

  [GeneratedRegex(@"\$\((?<config_expression>[^\)]+)\)")]
  private static partial Regex GetEmbeddedConfigExpressionRegex();

  [GeneratedRegex(@"^\$\((?<config_expression>[^\)]+)\)$")]
  private static partial Regex GetConfigExpressionRegex();
}
