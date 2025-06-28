using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginProvider : IPluginProvider, IAsyncDisposable
{
  private readonly IReadOnlyDictionary<string, IServiceProvider> _serviceProviders;

  public PluginProvider(IReadOnlyDictionary<string, IServiceProvider> serviceProviders)
  {
    _serviceProviders = serviceProviders;
  }

  public IPlugin GetPlugin(string name)
  {
    ArgumentException.ThrowIfNullOrWhiteSpace(name, nameof(name));
    var serviceProvider = _serviceProviders.GetValueOrDefault(name);
    if (serviceProvider is null)
    {
      throw new KeyNotFoundException($"Plugin with name '{name}' not found.");
    }

    var plugin = serviceProvider.GetKeyedService<IPlugin>(name);
    if (plugin is null)
    {
      throw new KeyNotFoundException($"Plugin with name '{name}' not found.");
    }

    return plugin;
  }

  public async ValueTask DisposeAsync()
  {
    foreach (var serviceProvider in _serviceProviders.Values)
    {
      switch (serviceProvider)
      {
        case IAsyncDisposable asyncDisposable:
          await asyncDisposable.DisposeAsync();
          break;
        case IDisposable disposable:
          disposable.Dispose();
          break;
      }
    }
  }
}
