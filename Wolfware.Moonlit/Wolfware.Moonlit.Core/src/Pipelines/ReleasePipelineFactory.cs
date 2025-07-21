using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Configuration.Abstractions;
using Wolfware.Moonlit.Core.Pipelines.Abstractions;
using Wolfware.Moonlit.Core.Plugins.Abstractions;

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
  private readonly IConditionEvaluator _conditionEvaluator;
  private readonly ILoggerProvider _loggerProvider;

  public ReleasePipelineFactory(
    IPluginsContextFactory pluginsContextFactory,
    IConfigurationFactory configurationFactory,
    IConditionEvaluator conditionEvaluator,
    ILoggerProvider loggerProvider
  )
  {
    _pluginsContextFactory = pluginsContextFactory;
    _configurationFactory = configurationFactory;
    _conditionEvaluator = conditionEvaluator;
    _loggerProvider = loggerProvider;
  }

  public async Task<ReleasePipeline> Create(ReleaseConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));
    var releaseConfiguration = this.GetReleaseConfiguration(configuration);
    var pluginsContext = await this._pluginsContextFactory.CreateContext(configuration.Plugins, releaseConfiguration)
      .ConfigureAwait(false);
    var middlewares = configuration.Stages.SelectMany(x => x.Value)
      .Select(x => new MiddlewareContext
      {
        Name = x.StepName,
        Middleware = pluginsContext.GetPlugin(x.PluginName).GetMiddleware(x.MiddlewareName),
        ContinueOnError = x.ContinueOnError,
        Condition = x.Condition,
        HaltIf = x.HaltIf,
        Configuration = x.Configuration
      })
      .ToList();
    var logger = this._loggerProvider.CreateLogger("Wolfware.Moonlit.Release");
    return new ReleasePipeline(
      pluginsContext,
      middlewares,
      this._configurationFactory,
      this._conditionEvaluator,
      logger
    );
  }

  private IConfiguration GetReleaseConfiguration(ReleaseConfiguration configuration)
  {
    var releaseConfiguration = new Dictionary<string, object?>();
    foreach (var variable in configuration.Variables)
    {
      releaseConfiguration[$"vars:{variable.Key}"] = variable.Value;
    }

    foreach (var argument in configuration.Arguments)
    {
      releaseConfiguration[$"args:{argument.Key}"] = argument.Value;
    }

    var baseConfiguration = this._configurationFactory.CreateBaseConfiguration();
    return this._configurationFactory.Create(releaseConfiguration, baseConfiguration);
  }
}
