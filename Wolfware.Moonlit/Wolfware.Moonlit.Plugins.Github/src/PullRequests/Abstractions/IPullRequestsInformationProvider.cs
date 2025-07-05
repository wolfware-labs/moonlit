using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.PullRequests.Configuration;

namespace Wolfware.Moonlit.Plugins.Github.PullRequests.Abstractions;

public interface IPullRequestsInformationProvider :
  IItemsInformationProvider<PullRequestsInformationFetchConfiguration>;
