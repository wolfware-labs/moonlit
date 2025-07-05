using Wolfware.Moonlit.Plugins.Github.Core.Models;

namespace Wolfware.Moonlit.Plugins.Github.Core.Abstractions;

public interface IItemsProvider<TItem>
{
  Task<IReadOnlyList<TItem>> GetItems(IGitHubContext context, FetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default);
}
