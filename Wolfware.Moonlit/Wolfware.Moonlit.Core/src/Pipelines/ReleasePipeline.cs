using System.Diagnostics;
using System.Text.Json;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Core.Pipelines;

/// <summary>
/// Represents a release pipeline responsible for executing a sequence of middleware components
/// in the context of a release process.
/// </summary>
public sealed class ReleasePipeline : IAsyncDisposable
{
  private readonly IPluginsContext _pluginsContext;
  private readonly IReadOnlyList<IMiddlewareContext> _middlewareContexts;
  private readonly IConfigurationFactory _configurationFactory;

  public ReleasePipeline(
    IPluginsContext pluginsContext,
    IReadOnlyList<IMiddlewareContext> middlewareContexts,
    IConfigurationFactory configurationFactory
  )
  {
    _pluginsContext = pluginsContext;
    _middlewareContexts = middlewareContexts;
    _configurationFactory = configurationFactory;
  }

  /// <summary>
  /// Executes the pipeline asynchronously with the provided context.
  /// </summary>
  /// <param name="context">The context containing configuration, logger, working directory, and cancellation token for the pipeline execution.</param>
  /// <returns>A task representing an asynchronous operation that returns the result of the pipeline execution, including success or failure state and any associated warnings or errors.</returns>
  public async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));

    var result = MiddlewareResult.Warning("No middlewares registered in the pipeline.");

    if (this._middlewareContexts.Count == 0)
    {
      return result;
    }

    IConfiguration configuration = this._configurationFactory.CreateBaseConfiguration();

    foreach (var middlewareContext in _middlewareContexts)
    {
      if (context.CancellationToken.IsCancellationRequested)
      {
        return MiddlewareResult.Failure("Pipeline execution was cancelled.");
      }

      try
      {
        context.Logger.LogInformation(">>> Start Step: {MiddlewareName} <<<", middlewareContext.Name);
        if (context.Logger.IsEnabled(LogLevel.Debug))
        {
          context.Logger.LogDebug("Middleware configuration: {Configuration}",
            JsonSerializer.Serialize(middlewareContext.Configuration, JsonSerializerOptions.Default));
        }

        var middlewareConfiguration = this._configurationFactory.Create(middlewareContext.Configuration, configuration);
        var stopwatch = Stopwatch.StartNew();
        result = await middlewareContext.Middleware.ExecuteAsync(context, middlewareConfiguration)
          .ConfigureAwait(false);
        stopwatch.Stop();
        context.Logger.LogInformation("<<< End Step: {MiddlewareName} [{ElapsedMilliseconds} ms] >>>",
          middlewareContext.Name, stopwatch.ElapsedMilliseconds);

        if (result.Warnings.Count > 0)
        {
          foreach (var warning in result.Warnings)
          {
            context.Logger.LogWarning(warning);
          }
        }

        if (!result.IsSuccessful)
        {
          return result;
        }

        var resultOutput = result.Output.ToDictionary(middlewareContext.Name);
        if (resultOutput.Count > 0)
        {
          configuration = this._configurationFactory.Create(resultOutput, configuration);
        }
      }
      catch (OperationCanceledException)
      {
        context.Logger.LogInformation("Pipeline execution was cancelled by the user.");
        return MiddlewareResult.Failure("Pipeline execution was cancelled by the user.");
      }
      catch (Exception ex)
      {
        context.Logger.LogError(ex, "An error occurred while executing middleware {MiddlewareName}.",
          middlewareContext.Name);
        return MiddlewareResult.Failure(
          $"An error occurred while executing middleware {middlewareContext.Name}: {ex.Message}");
      }
    }

    return result;
  }

  /// <inheritdoc />
  public ValueTask DisposeAsync()
  {
    return _pluginsContext.DisposeAsync();
  }
}
