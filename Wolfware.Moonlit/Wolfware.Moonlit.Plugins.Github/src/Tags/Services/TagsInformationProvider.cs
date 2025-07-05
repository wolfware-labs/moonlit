using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Tags.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Tags.Services;

public sealed class TagsInformationProvider : ITagsInformationProvider
{
  private readonly IGitHubContextProvider _gitHubContextProvider;

  public TagsInformationProvider(IGitHubContextProvider gitHubContextProvider)
  {
    _gitHubContextProvider = gitHubContextProvider;
  }

  public async Task<IReadOnlyDictionary<string, object>> GetInfo(
    ReleaseContext context,
    TagsInformationFetchConfiguration fetchConfiguration
  )
  {
    var gitHubContext = await this._gitHubContextProvider.GetCurrentContext(context);
    return new Dictionary<string, object>();
  }
}
