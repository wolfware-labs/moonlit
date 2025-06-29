namespace Wolfware.Moonlit.Core.Abstractions;

public interface IAssemblyPathResolver
{
  ValueTask<string> ResolvePath(Uri assemblyUri, CancellationToken cancellationToken = default);
}
