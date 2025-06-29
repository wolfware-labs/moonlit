using JetBrains.Annotations;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Plugins.Extensions;

/// <summary>
/// Provides extension methods for registering middleware components in an <see cref="IServiceCollection"/>.
/// </summary>
/// <remarks>
/// This class is designed to simplify the registration of middleware components that implement <see cref="IReleaseMiddleware"/>
/// within the dependency injection container.
/// </remarks>
[PublicAPI]
public static class ServiceCollectionExtensions
{
  /// <summary>
  /// Registers a middleware component of type <typeparamref name="TMiddleware"/> with the specified name
  /// in the <see cref="IServiceCollection"/> dependency injection container.
  /// </summary>
  /// <typeparam name="TMiddleware">The type of the middleware component to be registered, which must implement <see cref="IReleaseMiddleware"/>.</typeparam>
  /// <param name="services">The <see cref="IServiceCollection"/> to which the middleware will be registered.</param>
  /// <param name="name">The unique name that identifies the middleware instance in the pipeline.</param>
  /// <returns>The updated <see cref="IServiceCollection"/> after adding the middleware registration.</returns>
  /// <exception cref="ArgumentNullException">Thrown if the <paramref name="name"/> parameter is null.</exception>
  public static IServiceCollection AddMiddleware<TMiddleware>(this IServiceCollection services, string name)
    where TMiddleware : class, IReleaseMiddleware
  {
    ArgumentNullException.ThrowIfNull(name);
    services.AddKeyedSingleton<IReleaseMiddleware, TMiddleware>(name);
    return services;
  }
}
