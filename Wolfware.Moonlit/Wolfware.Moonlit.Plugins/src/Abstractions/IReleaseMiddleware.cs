using JetBrains.Annotations;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Abstractions;

/// <summary>
/// Represents a middleware component responsible for handling specific tasks within
/// the release pipeline process. Implementations of this interface define middleware
/// behaviors that operate on the release context and configuration provided during execution.
/// </summary>
[PublicAPI]
public interface IReleaseMiddleware
{
  /// <summary>
  /// Executes the middleware logic within the release pipeline using the provided context and configuration.
  /// </summary>
  /// <param name="context">The <see cref="ReleaseContext"/> representing the release pipeline's execution context.</param>
  /// <param name="configuration">The <see cref="IConfiguration"/> containing configuration settings specific to the middleware.</param>
  /// <returns>A <see cref="MiddlewareResult"/> indicating the result of the middleware execution, including success status, error message (if any), and any output data.</returns>
  Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration);
}
