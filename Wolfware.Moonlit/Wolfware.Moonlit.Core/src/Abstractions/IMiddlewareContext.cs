using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Abstractions;

/// <summary>
/// Represents the contextual data provided to middleware components during pipeline execution.
/// </summary>
/// <remarks>
/// This interface defines the contract for accessing essential contextual information, including middleware-specific logic
/// and configuration, which is required for the execution of a middleware component in a pipeline workflow.
/// </remarks>
public interface IMiddlewareContext
{
  /// <summary>
  /// Gets the middleware component associated with the current context.
  /// </summary>
  /// <remarks>
  /// The <see cref="Middleware"/> property represents the middleware logic to be executed as part
  /// of the pipeline. This property provides access to the specific middleware implementation
  /// that performs operations or transformations on the pipeline context and may delegate processing
  /// to the next middleware in the chain.
  /// </remarks>
  public IReleaseMiddleware Middleware { get; }

  /// <summary>
  /// Gets the configuration settings associated with the middleware context.
  /// </summary>
  /// <remarks>
  /// The <see cref="Configuration"/> property provides access to the application configuration settings
  /// required by the middleware during pipeline execution. It allows middleware components to retrieve
  /// and utilize environment-specific or application-specific configurations for their processing logic.
  /// </remarks>
  public IConfiguration Configuration { get; }
}
