using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;

namespace Wolfware.Moonlit.Plugins.Pipeline;

public abstract class ReleaseMiddleware<TConfiguration> : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.GetRequired<TConfiguration>();
    return ExecuteAsync(context, config);
  }

  public abstract Task<MiddlewareResult> ExecuteAsync(PipelineContext context, TConfiguration configuration);
}
