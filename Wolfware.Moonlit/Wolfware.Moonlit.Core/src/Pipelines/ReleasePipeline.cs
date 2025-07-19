using System.Diagnostics;
using System.Reflection;
using System.Text;
using System.Text.Json;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Pipelines;

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
        if (!this.CheckMiddlewareCondition(middlewareContext, configuration))
        {
          continue;
        }

        result = await this.ExecuteMiddlewareAsync(middlewareContext, context, configuration)
          .ConfigureAwait(false);

        this.LogWarnings(result);

        if (!result.IsSuccessful && !middlewareContext.ContinueOnError)
        {
          return result;
        }

        configuration = this.AppendOutputToConfiguration(middlewareContext.Name, configuration, result);

        if (this.CheckMiddlewareHaltCondition(middlewareContext, configuration))
        {
          break;
        }
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

  private bool CheckMiddlewareCondition(IMiddlewareContext middlewareContext, IConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(middlewareContext.Condition) ||
        !_conditionEvaluator.Evaluate(configuration.GetSection("output"), middlewareContext.Condition))
    {
      return true;
    }

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
    return false;
  }

  private async Task<MiddlewareResult> ExecuteMiddlewareAsync(IMiddlewareContext middlewareContext,
    ReleaseContext context, IConfiguration configuration)
  {
    var middlewareName = middlewareContext.Name;
    var middlewareType = middlewareContext.Middleware.GetType();
    var middlewareAssembly = middlewareType.Assembly;
    var middlewareAssemblyVersion = this.GetAssemblyVersion(middlewareType.Assembly);

    var headerBuilder = new StringBuilder();
    headerBuilder.Append($"Executing {middlewareName} (");

    if (!string.IsNullOrWhiteSpace(middlewareAssembly.GetName().Name))
    {
      headerBuilder.Append($"{middlewareAssembly.GetName().Name}.");
    }

    headerBuilder.Append(middlewareType.Name);

    if (!string.IsNullOrWhiteSpace(middlewareAssemblyVersion))
    {
      headerBuilder.Append($" v{middlewareAssemblyVersion}");
    }

    headerBuilder.Append(")");

    this._logger.LogInformation("===================================================");
    this._logger.LogInformation(headerBuilder.ToString());
    if (this._logger.IsEnabled(LogLevel.Debug))
    {
      this._logger.LogDebug("Configuration: {Configuration}",
        JsonSerializer.Serialize(middlewareContext.Configuration, JsonSerializerOptions.Default));
    }

    this._logger.LogInformation("===================================================");

    var middlewareConfiguration = this._configurationFactory.Create(middlewareContext.Configuration, configuration);
    var stopwatch = Stopwatch.StartNew();
    var result = await middlewareContext.Middleware.ExecuteAsync(context, middlewareConfiguration)
      .ConfigureAwait(false);
    stopwatch.Stop();
    this._logger.LogInformation("--------------------------------------------------");
    this._logger.LogInformation("{MiddlewareResult} - Execution time: {ElapsedMilliseconds} ms.",
      result.IsSuccessful ? "SUCCESS" : "FAILED", stopwatch.ElapsedMilliseconds);
    this._logger.LogInformation("--------------------------------------------------");
    this._logger.LogInformation("");
    this._logger.LogInformation("");

    return result;
  }

  private string? GetAssemblyVersion(Assembly middlewareTypeAssembly)
  {
    var fileVersionAttribute = middlewareTypeAssembly
      .GetCustomAttribute<AssemblyFileVersionAttribute>();

    if (fileVersionAttribute != null)
    {
      return fileVersionAttribute.Version;
    }

    var informationalVersionAttribute = middlewareTypeAssembly
      .GetCustomAttribute<AssemblyInformationalVersionAttribute>();

    if (informationalVersionAttribute != null)
    {
      return informationalVersionAttribute.InformationalVersion;
    }

    var version = middlewareTypeAssembly.GetName().Version;
    return version?.ToString();
  }

  private void LogWarnings(MiddlewareResult result)
  {
    if (result.Warnings.Count <= 0)
    {
      return;
    }

    foreach (var warning in result.Warnings)
    {
      this._logger.LogWarning(warning);
    }
  }

  private IConfiguration AppendOutputToConfiguration(string middlewareName, IConfiguration configuration,
    MiddlewareResult result)
  {
    var resultOutput = result.Output.ToDictionary(middlewareName);
    return resultOutput.Count > 0 ? this._configurationFactory.Create(resultOutput, configuration) : configuration;
  }

  private bool CheckMiddlewareHaltCondition(IMiddlewareContext middlewareContext, IConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(middlewareContext.StopOn) ||
        !_conditionEvaluator.Evaluate(configuration.GetSection("output"), middlewareContext.StopOn))
    {
      return false;
    }

    this._logger.LogInformation("---------------------------------------------------");
    this._logger.LogInformation(
      "Stopping pipeline execution after {MiddlewareName} due to stop condition met.",
      middlewareContext.Name);
    this._logger.LogInformation("Condition: {Condition}", middlewareContext.StopOn);
    this._logger.LogInformation("---------------------------------------------------");
    return true;
  }

  /// <inheritdoc />
  public ValueTask DisposeAsync()
  {
    return _pluginsContext.DisposeAsync();
  }
}
