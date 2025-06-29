using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Pipelines;

public class MiddlewareContext : IMiddlewareContext
{
  public required IReleaseMiddleware Middleware { get; init; }

  public required IConfiguration Configuration { get; init; }
}
