using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IConditionEvaluator
{
  public bool Evaluate(IConfigurationSection outputSection, string expression);
}

