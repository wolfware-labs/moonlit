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
  /// Gets the name associated with the current middleware context.
  /// </summary>
  /// <remarks>
  /// The <see cref="Name"/> property uniquely identifies the middleware context. It can be used
  /// to reference or differentiate between multiple middleware components within a pipeline.
  /// This property plays a critical role in configuration mapping and ensures proper execution
  /// of associated middleware logic.
  /// </remarks>
  public string Name { get; }

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
  /// Gets the configuration settings specific to the middleware context.
  /// </summary>
  /// <remarks>
  /// The <see cref="Configuration"/> property provides a dictionary of key-value pairs representing
  /// configuration settings for the middleware. These settings can be used to customize the behavior
  /// of the middleware during its execution within the pipeline. The property supports dynamic
  /// configuration and can be extended or modified as per the requirements of the pipeline flow.
  /// </remarks>
  public Dictionary<string, string?> Configuration { get; }
}
