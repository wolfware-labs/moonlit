using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins.Abstractions;

/// <summary>
/// Represents a plugin that provides middleware functionality and supports asynchronous disposal.
/// </summary>
/// <remarks>
/// The <see cref="IPlugin"/> interface allows for obtaining middleware components by name.
/// It also supports asynchronous resource cleanup via the <see cref="IAsyncDisposable"/> interface.
/// </remarks>
public interface IPlugin : IAsyncDisposable
{
  /// <summary>
  /// Retrieves an instance of middleware associated with the specified name.
  /// </summary>
  /// <param name="name">The name of the middleware to retrieve.</param>
  /// <returns>An instance of <see cref="IReleaseMiddleware"/> associated with the given name.</returns>
  /// <exception cref="KeyNotFoundException">Thrown if no middleware is found with the specified name.</exception>
  IReleaseMiddleware GetMiddleware(string name);
}
