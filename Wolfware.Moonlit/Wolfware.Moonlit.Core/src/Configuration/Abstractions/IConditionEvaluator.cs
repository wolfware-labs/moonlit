using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Core.Configuration.Abstractions;

public interface IConditionEvaluator
{
  public bool Evaluate(IConfigurationSection outputSection, string expression);
}
