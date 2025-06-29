using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IPlugin : IAsyncDisposable
{
  IReleaseMiddleware GetMiddleware(string name);
}
