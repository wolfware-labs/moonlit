namespace Wolfware.Moonlit.Core.Nuget.Abstractions;

public interface INugetPackageExtractor
{
  Task<bool> ExtractPackageContent(
    string repositoryId,
    string packageId,
    string version,
    string destinationFolder,
    CancellationToken cancellationToken = default);
}
