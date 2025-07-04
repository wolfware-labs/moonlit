using JetBrains.Annotations;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;

namespace Wolfware.Moonlit.Plugins.Pipeline;

/// <summary>
/// Represents an abstract base class for implementing middleware components in a release pipeline.
/// Middleware components inheriting from this class are required to handle specific tasks defined
/// within their implementation based on the provided release context and configuration.
/// </summary>
/// <typeparam name="TConfiguration">
/// The type of the configuration object used by the middleware. This type specifies the
/// required configuration structure needed for the middleware's logic execution.
/// </typeparam>
[PublicAPI]
public abstract class ReleaseMiddleware<TConfiguration> : IReleaseMiddleware
{
  /// <inheritdoc />
  public Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.GetRequired<TConfiguration>();
    return ExecuteAsync(context, config);
  }

  /// <summary>
  /// Executes middleware logic for the release pipeline with the specified context and configuration.
  /// </summary>
  /// <param name="context">The release context that provides necessary information for executing the middleware.</param>
  /// <param name="configuration">The middleware-specific configuration required for execution.</param>
  /// <returns>A task representing the asynchronous operation, containing the result of middleware execution.</returns>
  public abstract Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, TConfiguration configuration);
}
