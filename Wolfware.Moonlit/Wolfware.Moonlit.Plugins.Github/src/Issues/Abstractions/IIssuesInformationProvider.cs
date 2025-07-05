using Wolfware.Moonlit.Plugins.Github.Core.Abstractions;
using Wolfware.Moonlit.Plugins.Github.Issues.Configuration;
using Wolfware.Moonlit.Plugins.Github.Issues.Models;

namespace Wolfware.Moonlit.Plugins.Github.Issues.Abstractions;

public interface IIssuesInformationProvider : IItemsInformationProvider<IssuesInformationFetchConfiguration>;
