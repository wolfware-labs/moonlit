using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Plugins.Resolvers;

public sealed class HttpPathResolver : IAssemblyPathResolver
{
  public async ValueTask<string> ResolvePath(Uri assemblyUri, CancellationToken cancellationToken = default)
  {
    var tempPath = Path.GetTempFileName();
    using var client = new HttpClient();
    var assemblyContent = await client.GetByteArrayAsync(assemblyUri, cancellationToken);
    await File.WriteAllBytesAsync(tempPath, assemblyContent, cancellationToken);
    return tempPath;
  }
}
