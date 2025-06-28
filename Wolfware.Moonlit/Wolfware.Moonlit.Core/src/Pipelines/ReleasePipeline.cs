using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Core.Pipelines;

public sealed class ReleasePipeline
{
  private readonly IReadOnlyList<IPipelineMiddleware> _middlewares;

  public ReleasePipeline(IReadOnlyList<IPipelineMiddleware> middlewares)
  {
    _middlewares = middlewares ?? throw new ArgumentNullException(nameof(middlewares));
  }

  public async Task<PipelineResult> ExecuteAsync(PipelineContext context)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));

    var result = PipelineResult.Warning("No middlewares registered in the pipeline.");

    if (this._middlewares.Count == 0)
    {
      return result;
    }

    foreach (var middleware in _middlewares)
    {
      if (context.CancellationToken.IsCancellationRequested)
      {
        return PipelineResult.Failure("Pipeline execution was cancelled.");
      }

      try
      {
        result = await middleware.ExecuteAsync(context);

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
          middleware.GetType().Name);
        return PipelineResult.Failure(
          $"An error occurred while executing middleware {middleware.GetType().Name}: {ex.Message}");
      }
    }

    return result;
  }
}
