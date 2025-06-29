namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPluginPathResolver
{
  ValueTask<string> GetPluginPath(Uri pluginUrl, CancellationToken cancellationToken);
}
