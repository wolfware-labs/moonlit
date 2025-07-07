namespace Wolfware.Moonlit.Core.Abstractions;

public interface INugetPackageExtractor
{
  Task<bool> ExtractPackageContentAsync(
    string packageId,
    string version,
    string destinationFolder,
    CancellationToken cancellationToken = default);
}
