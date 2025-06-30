using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Pipelines;

/// <summary>
/// A factory for creating instances of <see cref="ReleasePipeline"/> based on the provided
/// <see cref="ReleaseConfiguration"/>.
/// </summary>
/// <remarks>
/// This factory uses an <see cref="IPluginsContextFactory"/> to create a plugins context
/// and an <see cref="IConfigurationFactory"/> to configure middlewares for the release pipeline.
/// </remarks>
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
        Name = x.StepName,
        Middleware = pluginsContext.GetPlugin(x.PluginName).GetMiddleware(x.MiddlewareName),
        Configuration = this._configurationFactory.Create(x.Configuration)
      })
      .ToList();
    return new ReleasePipeline(pluginsContext, middlewares, this._configurationFactory);
  }
}
