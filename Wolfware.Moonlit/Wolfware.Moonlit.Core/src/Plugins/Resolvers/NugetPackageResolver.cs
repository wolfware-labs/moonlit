using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins.Resolvers;

public sealed class NugetPackageResolver : IFilePathResolver
{
  public ValueTask<string> ResolvePath(Uri assemblyUri, CancellationToken cancellationToken = default)
  {
    throw new NotImplementedException();
  }
}
