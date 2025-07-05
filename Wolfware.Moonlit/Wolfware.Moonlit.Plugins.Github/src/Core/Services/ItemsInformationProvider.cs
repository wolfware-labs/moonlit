using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Core.Services;

public abstract class ItemsInformationProvider<TItemFetchConfiguration> :
  IItemsInformationProvider<TItemFetchConfiguration>
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public ItemsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    TItemFetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default
  )
  {
    var gitHubContext = await _gitHubContextProvider.GetCurrentContext(context)
      .ConfigureAwait(false);
    return await GetInfo(gitHubContext, fetchConfiguration, cancellationToken)
      .ConfigureAwait(false);
  }

  protected abstract Task<IReadOnlyDictionary<string, object>> GetInfo(
    IGitHubContext gitHubContext,
    TItemFetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken
  );
}
