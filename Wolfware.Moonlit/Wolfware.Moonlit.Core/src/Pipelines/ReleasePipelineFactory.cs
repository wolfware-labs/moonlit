using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Pipelines;

public sealed class ReleasePipelineFactory : IReleasePipelineFactory
{
  private readonly IPluginsContextFactory _pluginsContextFactory;

  public ReleasePipelineFactory(IPluginsContextFactory pluginsContextFactory)
  {
    _pluginsContextFactory = pluginsContextFactory;
  }

  public async Task<ReleasePipeline> Create(ReleaseConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var pluginsContext = await this._pluginsContextFactory.CreateContext(configuration.Plugins);
    var middlewares = configuration.Stages.SelectMany(x => x.Value)
      .Select(x => pluginsContext.GetPlugin(x.Plugin).GetMiddleware(x.Name))
      .ToList();
    return new ReleasePipeline(pluginsContext, middlewares);
  }
}
