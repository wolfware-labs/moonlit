using McMaster.NETCore.Plugins;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginRegistry : IDisposable
{
  private readonly IServiceCollection _services;
  private readonly List<PluginLoader> _pluginLoaders = [];

  public PluginRegistry(IServiceCollection services)
  {
    _services = services;
  }

  public async Task RegisterPlugin(PluginConfiguration configuration, CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(configuration);
    var assemblyPath = await GetAssemblyPath(configuration, cancellationToken);
    var pluginConfiguration = PluginRegistry.GetConfiguration(configuration);
    this._pluginLoaders.Add(Plugin.Load(assemblyPath, this._services, pluginConfiguration));
  }

  private async ValueTask<string> GetAssemblyPath(PluginConfiguration configuration,
    CancellationToken cancellationToken)
  {
    if (configuration.Url.Scheme == "file")
    {
      if (string.IsNullOrEmpty(configuration.Url.LocalPath))
      {
        throw new ArgumentException("Local path cannot be null or empty for file scheme.", nameof(configuration));
      }

      if (!File.Exists(configuration.Url.LocalPath))
      {
        throw new FileNotFoundException($"Assembly file not found at path: {configuration.Url.LocalPath}");
      }

      return configuration.Url.LocalPath;
    }

    if (configuration.Url.Scheme is "http" or "https")
    {
      var tempPath = Path.GetTempFileName();
      using var client = new HttpClient();
      var assemblyContent = await client.GetByteArrayAsync(configuration.Url, cancellationToken);
      await File.WriteAllBytesAsync(tempPath, assemblyContent, cancellationToken);
      return tempPath;
    }

    throw new NotSupportedException($"Unsupported URL scheme: {configuration.Url.Scheme}");
  }

  private static IConfiguration GetConfiguration(PluginConfiguration configuration)
  {
    return new ConfigurationBuilder()
      .AddInMemoryCollection(configuration.Configuration)
      .Build();
  }

  public void Dispose()
  {
    this._pluginLoaders.ForEach(loader =>
    {
      try
      {
        loader.Dispose();
      }
      catch (Exception ex)
      {
        // Log the exception if necessary
        Console.WriteLine($"Error disposing plugin loader: {ex.Message}");
      }
    });
  }
}
