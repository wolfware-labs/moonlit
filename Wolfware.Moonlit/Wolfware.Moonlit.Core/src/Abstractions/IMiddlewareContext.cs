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
  /// Gets a value indicating whether the middleware execution should continue when a failure occurs.
  /// </summary>
  /// <remarks>
  /// The <see cref="ContinueOnError"/> property determines whether the pipeline should proceed
  /// with the execution of subsequent middleware components when the current middleware encounters an error.
  /// Setting this to <c>true</c> allows the pipeline to continue regardless of failures, whereas setting it to <c>false</c>
  /// stops further execution and triggers appropriate error handling mechanisms.
  /// </remarks>
  public bool ContinueOnError { get; }

  /// <summary>
  /// Gets the condition that determines whether the middleware should be executed during a pipeline run.
  /// </summary>
  /// <remarks>
  /// The <see cref="ExecuteOn"/> property specifies an optional condition that can control the execution
  /// of the middleware. This condition is evaluated at runtime, allowing dynamic inclusion or exclusion
  /// of the middleware logic based on the specified criteria. If no condition is provided, the middleware
  /// is considered always eligible for execution.
  /// </remarks>
  public string? ExecuteOn { get; }

  /// <summary>
  /// Gets the condition specifying when the pipeline execution should stop after this middleware.
  /// </summary>
  /// <remarks>
  /// The <see cref="StopOn"/> property specifies an optional condition that, when evaluated to true,
  /// will halt the pipeline execution after this middleware completes. The condition is evaluated using
  /// a dynamic expression interpreter, with access to the pipeline's output configuration. If no condition
  /// is specified (null), the pipeline continues execution normally.
  /// </remarks>
  public string? StopOn { get; }

  /// <summary>
  /// Gets the configuration settings specific to the middleware context.
  /// </summary>
  /// <remarks>
  /// The <see cref="Configuration"/> property provides a dictionary of key-value pairs representing
  /// configuration settings for the middleware. These settings can be used to customize the behavior
  /// of the middleware during its execution within the pipeline. The property supports dynamic
  /// configuration and can be extended or modified as per the requirements of the pipeline flow.
  /// </remarks>
  public Dictionary<string, object?> Configuration { get; }
}
