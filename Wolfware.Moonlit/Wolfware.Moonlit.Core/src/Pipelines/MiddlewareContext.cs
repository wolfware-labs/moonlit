using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Pipelines;

/// <summary>
/// Represents the context passed to middleware during the execution of a pipeline.
/// </summary>
/// <remarks>
/// This class provides the necessary data and configuration for a middleware to execute within
/// the pipeline, including access to the associated middleware logic and its configuration settings.
/// It implements the <see cref="IMiddlewareContext"/> interface, ensuring adherence to the required contract.
/// </remarks>
public class MiddlewareContext : IMiddlewareContext
{
  /// <inheritdoc />
  public required string Name { get; init; }

  /// <inheritdoc />
  public required IReleaseMiddleware Middleware { get; init; }

  /// <inheritdoc />
  public required IConfiguration Configuration { get; init; }
}
