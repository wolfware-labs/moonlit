using Wolfware.Moonlit.Plugins.Github.Tags.Models;

namespace Wolfware.Moonlit.Plugins.Github.Tags.Configuration;

public sealed class TagsInformationFetchConfiguration
{
  public TagsFetchStrategy Strategy { get; set; } = TagsFetchStrategy.Latest;

  public string? FilterPattern { get; set; }
}
