using Wolfware.Moonlit.Plugins.Github.Commits.Models;

namespace Wolfware.Moonlit.Plugins.Github.Commits.Configuration;

public class CommitsInformationFetchConfiguration
{
  public CommitsFetchStrategy Strategy { get; set; } = CommitsFetchStrategy.None;
}
