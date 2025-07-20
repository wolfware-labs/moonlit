using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Core.FileSystem.Abstractions;
using Wolfware.Moonlit.Core.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins;

public sealed class PluginPathResolver : IPluginPathResolver
{
  private readonly IServiceProvider _serviceProvider;

  public PluginPathResolver(IServiceProvider serviceProvider)
  {
    _serviceProvider = serviceProvider;
  }

  public ValueTask<string> GetPluginPath(Uri pluginUrl, CancellationToken cancellationToken)
  {
    var resolver = this._serviceProvider.GetRequiredKeyedService<IFilePathResolver>(pluginUrl.Scheme);
    if (resolver == null)
    {
      throw new NotSupportedException($"Unsupported URL scheme: {pluginUrl.Scheme}");
    }

    return resolver.ResolvePath(pluginUrl, cancellationToken);
  }
}
