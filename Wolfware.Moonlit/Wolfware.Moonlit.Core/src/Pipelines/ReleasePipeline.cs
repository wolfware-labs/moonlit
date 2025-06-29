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

  public ReleasePipeline(IPluginsContext pluginsContext, IReadOnlyList<IMiddlewareContext> middlewareContexts,
    IConfigurationFactory configurationFactory)
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
  public async Task<PipelineResult> ExecuteAsync(PipelineContext context)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));

    var result = PipelineResult.Warning("No middlewares registered in the pipeline.");

    if (this._middlewareContexts.Count == 0)
    {
      return result;
    }

    IConfiguration configuration = new ConfigurationBuilder().AddEnvironmentVariables().Build();

    foreach (var middlewareContext in _middlewareContexts)
    {
      if (context.CancellationToken.IsCancellationRequested)
      {
        return PipelineResult.Failure("Pipeline execution was cancelled.");
      }

      try
      {
        var middlewareConfiguration = new ConfigurationBuilder()
          .AddConfiguration(configuration)
          .AddConfiguration(middlewareContext.Configuration)
          .Build();
        result = await middlewareContext.Middleware.ExecuteAsync(context, middlewareConfiguration)
          .ConfigureAwait(false);

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

        configuration =
          this._configurationFactory.Create(result.Output.ToDictionary(middlewareContext.Name), configuration);
      }
      catch (OperationCanceledException)
      {
        context.Logger.LogInformation("Pipeline execution was cancelled by the user.");
        return PipelineResult.Failure("Pipeline execution was cancelled by the user.");
      }
      catch (Exception ex)
      {
        context.Logger.LogError(ex, "An error occurred while executing middleware {MiddlewareName}.",
          middlewareContext.GetType().Name);
        return PipelineResult.Failure(
          $"An error occurred while executing middleware {middlewareContext.GetType().Name}: {ex.Message}");
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
