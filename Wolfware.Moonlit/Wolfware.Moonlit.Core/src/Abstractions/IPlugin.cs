using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPlugin
{
  IReleaseMiddleware GetMiddleware(string name);
}
