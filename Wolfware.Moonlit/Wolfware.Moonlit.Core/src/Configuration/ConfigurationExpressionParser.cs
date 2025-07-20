using System.Text.RegularExpressions;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Configuration.Abstractions;
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

    (var embedded, var configExpressions) = extractionResult.Value;

    return embedded
      ? ConfigurationExpressionParser.ParseTemplateExpression(expression, context, configExpressions)
      : ConfigurationExpressionParser.ParseConfigExpression(context, configExpressions[0]);
  }

  private static (bool Embedded, string[] configExpressions)? ExtractExpression(string expression)
  {
    var match = ConfigurationExpressionParser._configExpressionRegex.Match(expression);
    if (match.Success && match.Groups.TryGetValue("config_expression", out var configExpression))
    {
      return (false, [configExpression.Value]);
    }

    var matches = ConfigurationExpressionParser._embeddedConfigExpressionRegex.Matches(expression);
    if (!matches.Any(x => x.Success))
    {
      return null;
    }

    var expressions = matches
      .Where(x => x.Groups.TryGetValue("config_expression", out _))
      .Select(x => x.Groups["config_expression"].Value)
      .Distinct()
      .ToArray();
    return expressions.Length > 0 ? (true, expressions) : null;
  }

  private static object? ParseConfigExpression(IConfiguration context, string configExpression)
  {
    var configSection = context.GetSection(configExpression);
    return !configSection.Exists() ? null : configSection.AsObject();
  }

  private static string ParseTemplateExpression(string template, IConfiguration context, string[] configExpressions)
  {
    var expression = template;
    foreach (var configExpression in configExpressions)
    {
      var configSection = context.GetSection(configExpression);
      if (!configSection.Exists())
      {
        expression = expression.Replace($"$({configExpression})", string.Empty);
        continue;
      }

      var configValue = configSection.AsObject();
      expression = expression.Replace($"$({configExpression})", configValue?.ToString() ?? string.Empty);
    }

    return expression;
  }

  [GeneratedRegex(@"\$\((?<config_expression>[^\)]+)\)")]
  private static partial Regex GetEmbeddedConfigExpressionRegex();

  [GeneratedRegex(@"^\$\((?<config_expression>[^\)]+)\)$")]
  private static partial Regex GetConfigExpressionRegex();
}
