using NuGet.Common;
using NuGet.Configuration;
using NuGet.Packaging;
using NuGet.Protocol;
using NuGet.Protocol.Core.Types;
using NuGet.Versioning;

namespace Wolfware.Moonlit.Core.Nuget;

public class NugetDownloader
{
  public async Task DownloadPackageAndDependencies(string packageId, string version, string outputDirectory)
  {
    using var cache = new SourceCacheContext();
    var logger = NullLogger.Instance;
    var source = new PackageSource("https://api.nuget.org/v3/index.json");
    var repository = Repository.Factory.GetCoreV3(source);
    var resource = await repository.GetResourceAsync<FindPackageByIdResource>();
    var nugetVersion = NuGetVersion.Parse(version);
    HashSet<string> downloadedPackages = [];
    await NugetDownloader.DownloadPackageAndDependenciesRecursive(
      packageId,
      nugetVersion,
      outputDirectory,
      resource,
      cache,
      logger,
      downloadedPackages
    );
  }

  private static async Task DownloadPackageAndDependenciesRecursive(
    string packageId,
    NuGetVersion nugetVersion,
    string outputDirectory,
    FindPackageByIdResource resource,
    SourceCacheContext cache,
    ILogger logger,
    HashSet<string> downloadedPackages
  )
  {
    string packageKey = $"{packageId}.{nugetVersion}";
    if (downloadedPackages.Contains(packageKey))
    {
      return;
    }

    using var packageStream = new MemoryStream();
    bool success = await resource.CopyNupkgToStreamAsync(packageId, nugetVersion, packageStream, cache, logger,
      CancellationToken.None);
    if (!success)
    {
      throw new Exception($"Failed to download package {packageId} {nugetVersion}");
    }

    packageStream.Seek(0, SeekOrigin.Begin);
    using var packageReader = new PackageArchiveReader(packageStream);
    var files = await packageReader.GetFilesAsync(CancellationToken.None);
    foreach (var file in files)
    {
      if (!file.EndsWith(".dll", StringComparison.OrdinalIgnoreCase) &&
          !file.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
      {
        continue;
      }

      var outputPath = Path.Combine(outputDirectory, Path.GetFileName(file));
      Directory.CreateDirectory(outputDirectory);
      await using var fileStream = File.Create(outputPath);
      await using var entryStream = await packageReader.GetStreamAsync(file, CancellationToken.None);
      await entryStream.CopyToAsync(fileStream);
    }

    downloadedPackages.Add(packageKey);

    var dependencies = await packageReader.GetPackageDependenciesAsync(CancellationToken.None);
    foreach (var group in dependencies)
    {
      foreach (var dependency in group.Packages)
      {
        await NugetDownloader.DownloadPackageAndDependenciesRecursive(
          dependency.Id,
          dependency.VersionRange.MinVersion!,
          outputDirectory,
          resource,
          cache,
          logger,
          downloadedPackages
        );
      }
    }
  }
}
