using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Logging;
using ConsoleLoggerProvider = Wolfware.Moonlit.Cli.Logging.ConsoleLoggerProvider;

namespace Wolfware.Moonlit.Cli.Extensions;

public static class LoggingBuilderExtensions
{
  public static ILoggingBuilder AddCliConsole(this ILoggingBuilder builder)
  {
    builder.Services.TryAddEnumerable(ServiceDescriptor.Singleton<ILoggerProvider, ConsoleLoggerProvider>());
    return builder;
  }
}
