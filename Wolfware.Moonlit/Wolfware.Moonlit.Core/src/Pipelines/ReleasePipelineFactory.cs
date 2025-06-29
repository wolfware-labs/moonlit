using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Pipelines;

public sealed class ReleasePipelineFactory : IReleasePipelineFactory
{
  private readonly IPluginsContextFactory _pluginsContextFactory;
  private readonly IConfigurationFactory _configurationFactory;

  public ReleasePipelineFactory(IPluginsContextFactory pluginsContextFactory,
    IConfigurationFactory configurationFactory)
  {
    _pluginsContextFactory = pluginsContextFactory;
    _configurationFactory = configurationFactory;
  }

  public async Task<ReleasePipeline> Create(ReleaseConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var pluginsContext = await this._pluginsContextFactory.CreateContext(configuration.Plugins).ConfigureAwait(false);
    var middlewares = configuration.Stages.SelectMany(x => x.Value)
      .Select(x => new MiddlewareContext
      {
        Middleware = pluginsContext.GetPlugin(x.PluginName).GetMiddleware(x.MiddlewareName),
        Configuration = this._configurationFactory.Create(x.Configuration)
      })
      .ToList();
    return new ReleasePipeline(pluginsContext, middlewares);
  }
}
