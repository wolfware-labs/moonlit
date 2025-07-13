using System.Diagnostics;
using System.Text.Json;
using DynamicExpresso;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Extensions;
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
  private readonly IConditionEvaluator _conditionEvaluator;
  private readonly ILogger _logger;

  public ReleasePipeline(
    IPluginsContext pluginsContext,
    IReadOnlyList<IMiddlewareContext> middlewareContexts,
    IConfigurationFactory configurationFactory,
    IConditionEvaluator conditionEvaluator,
    ILogger logger
  )
  {
    _pluginsContext = pluginsContext;
    _middlewareContexts = middlewareContexts;
    _configurationFactory = configurationFactory;
    _conditionEvaluator = conditionEvaluator;
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
        if (!string.IsNullOrWhiteSpace(middlewareContext.Condition) &&
            _conditionEvaluator.Evaluate(configuration.GetSection("output"), middlewareContext.Condition))
        {
          this._logger.LogInformation("===================================================");
          this._logger.LogInformation(
            "Skipping {MiddlewareName} ({MiddlewareType}) due to condition not met.",
            middlewareContext.Name,
            middlewareContext.Middleware.GetType().Name
          );
          this._logger.LogInformation("Condition: {Condition}", middlewareContext.Condition);
          this._logger.LogInformation("===================================================");
          this._logger.LogInformation("");
          this._logger.LogInformation("");
          continue;
        }

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

        switch (result.IsSuccessful)
        {
          case false when !middlewareContext.ContinueOnError:
            return result;
          case true:
          {
            var resultOutput = result.Output.ToDictionary(middlewareContext.Name);
            if (resultOutput.Count > 0)
            {
              configuration = this._configurationFactory.Create(resultOutput, configuration);
            }

            break;
          }
        }

        if (string.IsNullOrWhiteSpace(middlewareContext.StopOn) ||
            !_conditionEvaluator.Evaluate(configuration.GetSection("output"), middlewareContext.StopOn))
        {
          continue;
        }

        this._logger.LogInformation("---------------------------------------------------");
        this._logger.LogInformation(
          "Stopping pipeline execution after {MiddlewareName} due to stop condition met.",
          middlewareContext.Name);
        this._logger.LogInformation("Condition: {Condition}", middlewareContext.StopOn);
        this._logger.LogInformation("---------------------------------------------------");
        break;
      }
      catch (OperationCanceledException)
      {
        return MiddlewareResult.Failure("Pipeline execution was cancelled by the user.");
      }
      catch (Exception ex)
      {
        if (!middlewareContext.ContinueOnError)
        {
          return MiddlewareResult.Failure(
            $"An error occurred while executing middleware {middlewareContext.Name}: {ex.Message}");
        }
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
