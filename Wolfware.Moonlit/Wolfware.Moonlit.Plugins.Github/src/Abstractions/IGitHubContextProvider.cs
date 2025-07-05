using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Abstractions;

public interface IGitHubContextProvider
{
  ValueTask<IGitHubContext> GetCurrentContext(ReleaseContext context);
}
