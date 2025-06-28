namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPluginProvider
{
  IPlugin GetPlugin(string name);
}
