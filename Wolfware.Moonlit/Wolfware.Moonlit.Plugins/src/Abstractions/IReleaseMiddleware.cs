using JetBrains.Annotations;
using Microsoft.Extensions.Configuration;
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
public interface IReleaseMiddleware
{
  /// <summary>
  /// Executes the middleware operation asynchronously.
  /// </summary>
  /// <param name="context">The pipeline context containing shared data, logging, and cancellation support.</param>
  /// <param name="configuration">The configuration settings to be used during the middleware's operation.</param>
  /// <returns>A <see cref="PipelineResult"/> object representing the outcome of the middleware operation.</returns>
  Task<PipelineResult> ExecuteAsync(PipelineContext context, IConfiguration configuration);
}
