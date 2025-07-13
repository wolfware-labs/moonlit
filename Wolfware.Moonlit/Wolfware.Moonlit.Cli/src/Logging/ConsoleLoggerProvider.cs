using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Wolfware.Moonlit.Cli.Logging;

public sealed class ConsoleLoggerProvider : ILoggerProvider
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
    var indentation = categoryName.Contains("Plugin", StringComparison.OrdinalIgnoreCase) ? "\t" : string.Empty;
    var showLevel = !categoryName.StartsWith("Wolfware.Moonlit", StringComparison.OrdinalIgnoreCase) ||
                    categoryName.Contains("Plugin", StringComparison.OrdinalIgnoreCase);
    return new ConsoleLogger(indentation, showLevel, () => _filterOptions);
  }

  public void Dispose()
  {
    _onChange?.Dispose();
  }
}
