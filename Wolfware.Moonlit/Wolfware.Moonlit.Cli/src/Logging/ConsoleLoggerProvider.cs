using System.Text;
using System.Text.RegularExpressions;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Wolfware.Moonlit.Cli.Logging;

public partial class ConsoleLoggerProvider : ILoggerProvider
{
  private readonly IDisposable? _onChange;
  private LoggerFilterOptions _filterOptions;

  public ConsoleLoggerProvider(IOptionsMonitor<LoggerFilterOptions> optionsMonitor)
  {
    _filterOptions = optionsMonitor.CurrentValue;
    _onChange = optionsMonitor.OnChange(opt => _filterOptions = opt);
  }

  public ILogger CreateLogger(string categoryName)
  {
    return new ConsoleLogger(this.GetCategory(categoryName), () => _filterOptions);
  }

  public void Dispose()
  {
    _onChange?.Dispose();
  }

  private string GetCategory(string categoryName)
  {
    if (ConsoleLoggerProvider.CategoryRegex().Match(categoryName) is not {Success: true} match)
    {
      return categoryName;
    }

    string sdkName = match.Groups["context"].Value;
    string className = match.Groups["className"].Value;

    var categoryBuilder = new StringBuilder();
    if (!string.IsNullOrWhiteSpace(sdkName))
    {
      categoryBuilder.Append(sdkName);
      categoryBuilder.Append('.');
    }

    if (!string.IsNullOrWhiteSpace(className))
    {
      categoryBuilder.Append(className);
    }
    else if (categoryBuilder.Length > 0)
    {
      categoryBuilder.Length--;
    }

    return categoryBuilder.ToString();
  }

  [GeneratedRegex(@"^(?:Wolfware\.Moonlit\.?(?:Core\.)?(?:Plugins\.)?)(?<context>.*?)?(\..*)?\.(?<className>[^\.]*)$")]
  private static partial Regex CategoryRegex();
}
