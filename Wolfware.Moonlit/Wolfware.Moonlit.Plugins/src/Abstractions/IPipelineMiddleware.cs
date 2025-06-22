using JetBrains.Annotations;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Abstractions;

/// <summary>
/// Represents the contract for a middleware component in a pipeline.
/// </summary>
/// <remarks>
/// Middleware components in a pipeline are responsible for performing specific operations or processing on the pipeline context.
/// They can also delegate execution to the next component in the chain.
/// </remarks>
[PublicAPI]
public interface IPipelineMiddleware
{
  /// <summary>
  /// Executes a pipeline middleware operation with the provided pipeline context.
  /// </summary>
  /// <param name="context">The <see cref="PipelineContext"/> that contains the state and data for the current pipeline execution.</param>
  /// <returns>A <see cref="Task{PipelineResult}"/> representing the asynchronous operation, including the result of the pipeline execution.</returns>
  Task<PipelineResult> ExecuteAsync(PipelineContext context);
}
