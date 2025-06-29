namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPluginsContext : IAsyncDisposable
{
  IPlugin GetPlugin(string name);
}
