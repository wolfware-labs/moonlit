using Wolfware.Moonlit.Plugins.Github.Core.Models;

namespace Wolfware.Moonlit.Plugins.Github.Core.Configuration;

public class GetGitInformationConfiguration
{
  public FetchConfiguration? Branches { get; set; }

  public FetchConfiguration? Tags { get; set; }

  public FetchConfiguration? Commits { get; set; }

  public FetchConfiguration? PullRequests { get; set; }

  public FetchConfiguration? Issues { get; set; }
}
