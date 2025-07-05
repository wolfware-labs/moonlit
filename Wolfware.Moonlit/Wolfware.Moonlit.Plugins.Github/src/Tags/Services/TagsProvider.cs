using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Core.Models;
using Wolfware.Moonlit.Plugins.Github.Tags.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Models;

namespace Wolfware.Moonlit.Plugins.Github.Tags.Services;

public sealed class TagsProvider : ITagsProvider
{
  public Task<IReadOnlyList<GitHubTag>> GetItems(IGitHubContext context, FetchConfiguration fetchConfiguration,
    CancellationToken cancellationToken = default)
  {
    throw new NotImplementedException();
  }
}
