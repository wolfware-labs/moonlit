namespace Wolfware.Moonlit.Core.Abstractions;

public interface INugetPackageExtractor
{
  Task<bool> ExtractPackageContent(
    string packageId,
    string version,
    string destinationFolder,
    CancellationToken cancellationToken = default);
}
