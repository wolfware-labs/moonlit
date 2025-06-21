using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Plugins.ConventionalCommits;

internal sealed class VersionCalculator : IVersionCalculator
{
  public string Name => "Conventional Commits Version Calculator";
}
