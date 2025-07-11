using Microsoft.Extensions.Logging;
using Spectre.Console;

namespace Wolfware.Moonlit.Cli.Logging;

public sealed class ConsoleLogger : ILogger
{
  private readonly string _indentation;
  private readonly bool _showLevel;
  private readonly Func<LoggerFilterOptions> _filterAccessor;

  public ConsoleLogger(string indentation, bool showLevel, Func<LoggerFilterOptions> getFilters)
  {
    _indentation = indentation;
    _showLevel = showLevel;
    _filterAccessor = getFilters;
  }

  public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

  public bool IsEnabled(LogLevel logLevel)
  {
    var filters = _filterAccessor().Rules;

    foreach (var rule in filters)
    {
      var providerMatches = rule.ProviderName is null or nameof(ConsoleLoggerProvider);
      var categoryMatches = rule.CategoryName == null || _indentation.StartsWith(rule.CategoryName);

      if (providerMatches && categoryMatches)
        return logLevel >= rule.LogLevel;
    }

    return true;
  }

  public void Log<TState>(LogLevel logLevel, EventId eventId, TState state,
    Exception? exception, Func<TState, Exception?, string> formatter)
  {
    if (!IsEnabled(logLevel))
      return;

    var message = formatter(state, exception);
    var levelLabel = string.Empty;
    if (this._showLevel)
    {
      levelLabel = logLevel switch
      {
        LogLevel.Trace => "[grey]TRACE[/]",
        LogLevel.Debug => "[blue]DEBUG[/]",
        LogLevel.Information => "[green]INFO[/]",
        LogLevel.Warning => "[yellow]WARN[/]",
        LogLevel.Error => "[red]ERROR[/]",
        LogLevel.Critical => "[bold red]CRITICAL[/]",
        _ => "[white]UNKNOWN[/]"
      };
    }

    AnsiConsole.MarkupLine(
      $"[steelblue][[[/][khaki3]{DateTime.Now:HH:mm:ss}[/][steelblue]]][/] {_indentation} {levelLabel} {Markup.Escape(message)}");

    if (exception != null)
    {
      AnsiConsole.WriteException(exception);
    }
  }
}
