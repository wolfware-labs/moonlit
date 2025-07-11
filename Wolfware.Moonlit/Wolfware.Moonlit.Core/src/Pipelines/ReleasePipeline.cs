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
  private readonly ILogger _logger;

  public ReleasePipeline(
    IPluginsContext pluginsContext,
    IReadOnlyList<IMiddlewareContext> middlewareContexts,
    IConfigurationFactory configurationFactory,
    ILogger logger
  )
  {
    _pluginsContext = pluginsContext;
    _middlewareContexts = middlewareContexts;
    _configurationFactory = configurationFactory;
    _logger = logger;
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
        this._logger.LogInformation("===================================================");
        this._logger.LogInformation("Executing {MiddlewareName} ({MiddlewareType})", middlewareContext.Name,
          middlewareContext.Middleware.GetType().Name);
        if (this._logger.IsEnabled(LogLevel.Debug))
        {
          this._logger.LogDebug("Configuration: {Configuration}",
            JsonSerializer.Serialize(middlewareContext.Configuration, JsonSerializerOptions.Default));
        }

        this._logger.LogInformation("===================================================");

        var middlewareConfiguration = this._configurationFactory.Create(middlewareContext.Configuration, configuration);
        var stopwatch = Stopwatch.StartNew();
        result = await middlewareContext.Middleware.ExecuteAsync(context, middlewareConfiguration)
          .ConfigureAwait(false);
        stopwatch.Stop();
        this._logger.LogInformation("--------------------------------------------------");
        this._logger.LogInformation("{MiddlewareResult} - Execution time: {ElapsedMilliseconds} ms.",
          result.IsSuccessful ? "SUCCESS" : "FAILED", stopwatch.ElapsedMilliseconds);
        this._logger.LogInformation("--------------------------------------------------");
        this._logger.LogInformation("");
        this._logger.LogInformation("");

        if (result.Warnings.Count > 0)
        {
          foreach (var warning in result.Warnings)
          {
            this._logger.LogWarning(warning);
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
        this._logger.LogInformation("Pipeline execution was cancelled by the user.");
        return MiddlewareResult.Failure("Pipeline execution was cancelled by the user.");
      }
      catch (Exception ex)
      {
        this._logger.LogError(ex, "An error occurred while executing middleware {MiddlewareName}.",
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
