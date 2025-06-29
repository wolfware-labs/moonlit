using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IMiddlewareContext
{
  public IReleaseMiddleware Middleware { get; }

  public IConfiguration Configuration { get; }
}
