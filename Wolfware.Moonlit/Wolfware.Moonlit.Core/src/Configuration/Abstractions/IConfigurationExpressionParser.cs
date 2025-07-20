using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Core.Configuration.Abstractions;

public interface IConfigurationExpressionParser
{
  object? ParseExpression(string expression, IConfiguration context);
}
