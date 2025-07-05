using Wolfware.Moonlit.Plugins.Github.Tags.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Tags.Services;

public sealed class TagsInformationProvider : ITagsInformationProvider
{
  public Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    TagsInformationFetchConfiguration fetchConfiguration
  )
  {
    throw new NotImplementedException();
  }
}
