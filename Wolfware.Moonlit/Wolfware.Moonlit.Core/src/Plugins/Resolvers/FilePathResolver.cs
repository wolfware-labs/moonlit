using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins.Resolvers;

public sealed class FilePathResolver : IFilePathResolver
{
  public ValueTask<string> ResolvePath(Uri assemblyUri, CancellationToken cancellationToken = default)
  {
    if (string.IsNullOrEmpty(assemblyUri.LocalPath))
    {
      throw new ArgumentException("Local path cannot be null or empty for file scheme.", nameof(assemblyUri));
    }

    if (!File.Exists(assemblyUri.LocalPath))
    {
      throw new FileNotFoundException($"Assembly not found at path: {assemblyUri.LocalPath}");
    }

    return ValueTask.FromResult(assemblyUri.LocalPath);
  }
}
