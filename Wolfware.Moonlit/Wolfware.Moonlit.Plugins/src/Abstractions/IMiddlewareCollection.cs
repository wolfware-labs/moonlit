namespace Wolfware.Moonlit.Plugins.Abstractions;

/// <summary>
/// Represents a collection of middleware components, enabling middleware registration
/// and management in a pipeline architecture. Provides functionality to add middleware components
/// to the collection for execution in a pipeline.
/// </summary>
public interface IMiddlewareCollection
{
  /// <summary>
  /// Adds a middleware component to the collection for execution in the pipeline.
  /// </summary>
  /// <typeparam name="TMiddleware">The type of the middleware to add, which must implement <see cref="IReleaseMiddleware"/>.</typeparam>
  /// <param name="name">The unique name of the middleware, used to identify it within the collection.</param>
  /// <returns>An updated instance of <see cref="IMiddlewareCollection"/> that includes the newly added middleware.</returns>
  IMiddlewareCollection Add<TMiddleware>(string name) where TMiddleware : class, IReleaseMiddleware;
}
