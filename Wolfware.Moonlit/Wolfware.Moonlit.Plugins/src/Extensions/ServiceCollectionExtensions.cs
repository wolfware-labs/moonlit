using JetBrains.Annotations;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Extensions;

/// <summary>
/// Provides extension methods for the <see cref="IServiceCollection"/> type to facilitate
/// the registration and configuration of middleware components in an application.
/// </summary>
[PublicAPI]
public static class ServiceCollectionExtensions
{
  /// <summary>
  /// Adds middleware components to the service collection based on the provided configuration action.
  /// </summary>
  /// <param name="services">The <see cref="IServiceCollection"/> to which the middleware components will be added.</param>
  /// <param name="configure">An <see cref="Action{T}"/> to configure the middleware collection by adding middleware components.</param>
  /// <returns>The updated <see cref="IServiceCollection"/> instance with the middleware components registered.</returns>
  public static IServiceCollection AddMiddlewares(this IServiceCollection services,
    Action<IMiddlewareCollection> configure)
  {
    ArgumentNullException.ThrowIfNull(services);
    ArgumentNullException.ThrowIfNull(configure);
    var middlewareCollection = new MiddlewareCollection(services);
    configure(middlewareCollection);
    return services;
  }
}
