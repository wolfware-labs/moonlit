using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.SemanticRelease.Middlewares;

public sealed class GenerateChangelogs : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
