using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Plugins.Pipelines;

internal sealed class MiddlewareCollection : IMiddlewareCollection
{
  private readonly IServiceCollection _services;

  public MiddlewareCollection(IServiceCollection services)
  {
    _services = services;
  }

  public IMiddlewareCollection Add<TMiddleware>(string name) where TMiddleware : class, IReleaseMiddleware
  {
    ArgumentNullException.ThrowIfNull(name);
    this._services.AddKeyedSingleton<IReleaseMiddleware, TMiddleware>(name);
    return this;
  }
}
