using JetBrains.Annotations;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;

namespace Wolfware.Moonlit.Plugins.Abstractions;

/// <summary>
/// Defines the contract for initializing and configuring a plugin.
/// </summary>
[PublicAPI]
public interface IPluginStartup
{
  /// <summary>
  /// Configures services and settings required by the plugin.
  /// </summary>
  /// <param name="services">The collection of service descriptors used to register services.</param>
  /// <param name="configuration">The configuration for the application to retrieve settings and values.</param>
  void Configure(IServiceCollection services, IConfiguration configuration);
}
