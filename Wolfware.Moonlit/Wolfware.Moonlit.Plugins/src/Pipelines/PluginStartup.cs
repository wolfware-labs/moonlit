using JetBrains.Annotations;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;

namespace Wolfware.Moonlit.Plugins.Pipelines;

/// <summary>
/// Provides a base implementation for configuring and initializing a plugin.
/// Implements the <see cref="IPluginStartup"/> interface for plugin-specific service registration and middleware setup.
/// </summary>
[PublicAPI]
public abstract class PluginStartup : IPluginStartup
{
  /// <summary>
  /// Configures the services and middleware required for the plugin.
  /// </summary>
  /// <param name="services">The service collection to which the plugin services and middleware are added.</param>
  /// <param name="configuration">The configuration settings required for the plugin initialization.</param>
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(services);
    ArgumentNullException.ThrowIfNull(configuration);

    this.ConfigurePlugin(services, configuration);
    services.AddMiddlewares(this.AddMiddlewares);
  }

  /// <summary>
  /// Adds the necessary services and dependencies required for the plugin.
  /// </summary>
  /// <param name="services">The service collection used for registering the plugin's services and dependencies.</param>
  /// <param name="configuration">The configuration instance providing settings for the plugin initialization.</param>
  protected virtual void ConfigurePlugin(IServiceCollection services, IConfiguration configuration) { }


  /// <summary>
  /// Adds middleware components to the middleware collection to configure the pipeline
  /// for the plugin's execution.
  /// </summary>
  /// <param name="middlewares">The middleware collection used for adding and managing middleware components.</param>
  protected abstract void AddMiddlewares(IMiddlewareCollection middlewares);
}
