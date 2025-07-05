using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

public interface IItemsInformationProvider<in TItemFetchConfiguration>
{
  Task PopulateFetchContext(
    ReleaseContext releaseContext,
    TItemFetchConfiguration fetchConfiguration,
    FetchContext fetchContext
  );
}
