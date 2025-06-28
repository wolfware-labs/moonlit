using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Plugins;

namespace Wolfware.Moonlit.Core.Pipelines;

public sealed class ReleasePipelineFactory
{
  public static ReleasePipeline Create(
    PluginsContext pluginsContext,
    MiddlewareConfiguration[] middlewareConfigurations
  )
  {
    ArgumentNullException.ThrowIfNull(pluginsContext, nameof(pluginsContext));
    ArgumentNullException.ThrowIfNull(middlewareConfigurations, nameof(middlewareConfigurations));

    var middlewares = middlewareConfigurations
      .Select(x => pluginsContext.PluginProvider.GetPlugin(x.Plugin).GetMiddleware(x.Name))
      .ToList();
    return new ReleasePipeline(middlewares);
  }
}
