using Microsoft.Extensions.Logging;
using Spectre.Console;

namespace Wolfware.Moonlit.Cli.Logging;

public sealed class ConsoleLogger : ILogger
{
  private readonly string _categoryName;
  private readonly Func<LoggerFilterOptions> _filterAccessor;

  public ConsoleLogger(string categoryName, Func<LoggerFilterOptions> getFilters)
  {
    _categoryName = categoryName;
    _filterAccessor = getFilters;
  }

  public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

  public bool IsEnabled(LogLevel logLevel)
  {
    var filters = _filterAccessor().Rules;

    // Find the first matching rule:
    foreach (var rule in filters)
    {
      // rule.ProviderName == null means “any provider”
      var providerMatches = rule.ProviderName is null or nameof(ConsoleLoggerProvider);
      var categoryMatches = rule.CategoryName == null || _categoryName.StartsWith(rule.CategoryName);

      if (providerMatches && categoryMatches)
        return logLevel >= rule.LogLevel;
    }

    // No rule matched? apply default (true)
    return true;
  }

  public void Log<TState>(LogLevel logLevel, EventId eventId, TState state,
    Exception? exception, Func<TState, Exception?, string> formatter)
  {
    if (!IsEnabled(logLevel))
      return;

    var message = formatter(state, exception);
    var levelLabel = logLevel switch
    {
      LogLevel.Trace => "[grey]TRACE[/]",
      LogLevel.Debug => "[blue]DEBUG[/]",
      LogLevel.Information => "[green]INFO[/]",
      LogLevel.Warning => "[yellow]WARN[/]",
      LogLevel.Error => "[red]ERROR[/]",
      LogLevel.Critical => "[bold red]CRITICAL[/]",
      _ => "[white]UNKNOWN[/]"
    };

    AnsiConsole.MarkupLine(
      $"[steelblue][[[/][khaki3]{DateTime.Now:HH:mm:ss}[/][steelblue]]][/] [gray][[{_categoryName}]][/] {levelLabel} {Markup.Escape(message)}");

    if (exception != null)
    {
      AnsiConsole.WriteException(exception);
    }
  }
}
