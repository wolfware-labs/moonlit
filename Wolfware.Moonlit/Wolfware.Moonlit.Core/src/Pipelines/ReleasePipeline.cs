using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Core.Pipelines;

public sealed class ReleasePipeline : IAsyncDisposable
{
  private readonly IPluginsContext _pluginsContext;
  private readonly IReadOnlyList<IMiddlewareContext> _middlewareContexts;

  public ReleasePipeline(IPluginsContext pluginsContext, IReadOnlyList<IMiddlewareContext> middlewareContexts)
  {
    _pluginsContext = pluginsContext;
    _middlewareContexts = middlewareContexts;
  }

  public async Task<PipelineResult> ExecuteAsync(PipelineContext context)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));

    var result = PipelineResult.Warning("No middlewares registered in the pipeline.");

    if (this._middlewareContexts.Count == 0)
    {
      return result;
    }

    foreach (var middlewareContext in _middlewareContexts)
    {
      if (context.CancellationToken.IsCancellationRequested)
      {
        return PipelineResult.Failure("Pipeline execution was cancelled.");
      }

      try
      {
        result = await middlewareContext.Middleware.ExecuteAsync(context, middlewareContext.Configuration)
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

  public async ValueTask DisposeAsync()
  {
    await _pluginsContext.DisposeAsync();
  }
}
