using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

public interface IGitHubContextProvider
{
  ValueTask<IGitHubContext> GetCurrentContext(ReleaseContext context);
}
