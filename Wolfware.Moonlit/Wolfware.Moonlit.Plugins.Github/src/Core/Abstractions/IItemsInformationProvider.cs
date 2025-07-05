using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

public interface IItemsInformationProvider<in TItemFetchConfiguration>
{
  Task<IReadOnlyDictionary<string, object>> GetInfo(ReleaseContext context, TItemFetchConfiguration fetchConfiguration);
}
