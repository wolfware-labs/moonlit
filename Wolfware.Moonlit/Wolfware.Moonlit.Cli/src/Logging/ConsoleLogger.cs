using Microsoft.Extensions.Logging;
using Spectre.Console;

namespace Wolfware.Moonlit.Cli.Logging;

public sealed class ConsoleLogger : ILogger
{
  private readonly bool _verbose;

  public ConsoleLogger(bool verbose = false)
  {
    _verbose = verbose;
  }

  public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

  public bool IsEnabled(LogLevel logLevel)
  {
    return logLevel switch
    {
      LogLevel.Trace => _verbose,
      LogLevel.Debug => _verbose,
      LogLevel.Information => true,
      LogLevel.Warning => true,
      LogLevel.Error => true,
      LogLevel.Critical => true,
      _ => false
    };
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
      $"[blue][[[/][orange3]{DateTime.Now:HH:mm:ss}[/][blue]]][/] {levelLabel} {Markup.Escape(message)}");

    if (exception != null)
    {
      AnsiConsole.WriteException(exception);
    }
  }
}
