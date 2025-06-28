using McMaster.NETCore.Plugins;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Configuration;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginRegistry : IDisposable
{
  private readonly List<PluginLoader> _pluginLoaders = [];

  public async Task RegisterPlugin(IServiceCollection services, PluginConfiguration configuration,
    CancellationToken cancellationToken = default)
  {
    ArgumentNullException.ThrowIfNull(configuration);
    var assemblyPath = await GetAssemblyPath(configuration.Url, cancellationToken);
    var pluginConfiguration = PluginRegistry.GetConfiguration(configuration);
    this._pluginLoaders.Add(Plugin.Load(assemblyPath, services, pluginConfiguration));
    services.AddKeyedSingleton<IPlugin, Plugin>(configuration.Name);
  }

  private async ValueTask<string> GetAssemblyPath(Uri pluginUrl, CancellationToken cancellationToken)
  {
    if (pluginUrl.Scheme == "file")
    {
      if (string.IsNullOrEmpty(pluginUrl.LocalPath))
      {
        throw new ArgumentException("Local path cannot be null or empty for file scheme.", nameof(pluginUrl));
      }

      if (!File.Exists(pluginUrl.LocalPath))
      {
        throw new FileNotFoundException($"Plugin assembly not found at path: {pluginUrl.LocalPath}");
      }

      return pluginUrl.LocalPath;
    }

    if (pluginUrl.Scheme is "http" or "https")
    {
      var tempPath = Path.GetTempFileName();
      using var client = new HttpClient();
      var assemblyContent = await client.GetByteArrayAsync(pluginUrl, cancellationToken);
      await File.WriteAllBytesAsync(tempPath, assemblyContent, cancellationToken);
      return tempPath;
    }

    throw new NotSupportedException($"Unsupported URL scheme: {pluginUrl.Scheme}");
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
