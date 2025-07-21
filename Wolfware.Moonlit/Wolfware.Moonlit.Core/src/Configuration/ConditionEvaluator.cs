using DynamicExpresso;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Configuration.Abstractions;
using Wolfware.Moonlit.Core.Extensions;

namespace Wolfware.Moonlit.Core.Configuration;

public sealed class ConditionEvaluator : IConditionEvaluator
{
  private readonly Interpreter _interpreter = new(InterpreterOptions.DefaultCaseInsensitive);

  public bool Evaluate(IConfigurationSection outputSection, string expression)
  {
    this._interpreter.SetVariable("Output", outputSection.AsObject());
    return this._interpreter.Eval(expression) is true;
  }
}
