using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Github.Abstractions;

public interface IGitHubContextProvider
{
  ValueTask<IGitHubContext> GetCurrentContext(ReleaseContext context);
}
