using Microsoft.Extensions.Logging;
using NuGet.Common;
using NuGet.Configuration;
using NuGet.Packaging;
using NuGet.Packaging.Core;
using NuGet.Protocol;
using NuGet.Protocol.Core.Types;
using NuGet.Versioning;
using Wolfware.Moonlit.Core.Abstractions;

namespace Wolfware.Moonlit.Core.Nuget;

public sealed class NugetPackageExtractor : INugetPackageExtractor
{
  private readonly ILogger<NugetPackageExtractor> _logger;
  private readonly SourceCacheContext _cacheContext;
  private readonly List<SourceRepository> _repositories;

  public NugetPackageExtractor(ILogger<NugetPackageExtractor> logger)
  {
    this._logger = logger;
    this._cacheContext = new SourceCacheContext();

    var packageSource = new PackageSource("https://api.nuget.org/v3/index.json");
    var sourceRepository = Repository.Factory.GetCoreV3(packageSource);
    this._repositories = [sourceRepository];
  }

  public Task<bool> ExtractPackageContent(
    string packageId,
    string version,
    string destinationFolder,
    CancellationToken cancellationToken = default)
  {
    var downloadedPackages = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
    return ExtractPackageContent(packageId, version, destinationFolder, downloadedPackages, cancellationToken);
  }

  private async Task<bool> ExtractPackageContent(
    string packageId,
    string version,
    string destinationFolder,
    HashSet<string> downloadedPackages,
    CancellationToken cancellationToken
  )
  {
    if (downloadedPackages.Contains($"{packageId}.{version}"))
    {
      return true;
    }

    try
    {
      var packageIdentity = new PackageIdentity(packageId, NuGetVersion.Parse(version));
      var downloadResource = await _repositories[0].GetResourceAsync<DownloadResource>(cancellationToken);

      var downloadResult = await downloadResource.GetDownloadResourceResultAsync(
        packageIdentity,
        new PackageDownloadContext(_cacheContext),
        SettingsUtility.GetGlobalPackagesFolder(Settings.LoadDefaultSettings(null)),
        NullLogger.Instance,
        cancellationToken);

      if (downloadResult?.PackageReader == null)
      {
        this._logger.LogError("Failed to download package {PackageId} {Version}", packageId, version);
        return false;
      }

      using var packageReader = downloadResult.PackageReader;
      await NugetPackageExtractor.ExtractCompatibleContentAsync(packageReader, destinationFolder);
      downloadedPackages.Add($"{packageId}.{version}");
      var deps = await packageReader.GetPackageDependenciesAsync(cancellationToken);
      await Parallel.ForEachAsync(NugetRuntime.GetMostCompatibleGroup(deps.ToArray())?.Packages ?? [],
        cancellationToken, async (dep, ct) =>
        {
          await ExtractPackageContent(
            dep.Id,
            dep.VersionRange.MinVersion!.ToString(),
            destinationFolder,
            downloadedPackages,
            ct
          );
        }
      );

      return true;
    }
    catch (Exception)
    {
      return false;
    }
  }

  private static async Task ExtractCompatibleContentAsync(PackageReaderBase packageReader, string destinationFolder)
  {
    Directory.CreateDirectory(destinationFolder);
    await NugetPackageExtractor.ExtractLibFilesAsync(packageReader, destinationFolder);
    await NugetPackageExtractor.ExtractRuntimeFilesAsync(packageReader, destinationFolder);
    await NugetPackageExtractor.ExtractContentFilesAsync(packageReader, destinationFolder);
    await NugetPackageExtractor.ExtractBuildFilesAsync(packageReader, destinationFolder);
  }

  private static async Task ExtractLibFilesAsync(PackageReaderBase packageReader, string destinationFolder)
  {
    var libItems = await packageReader.GetLibItemsAsync(CancellationToken.None);
    var compatibleLib = NugetRuntime.GetMostCompatibleGroup(libItems.ToArray());

    if (compatibleLib != null)
    {
      foreach (var file in compatibleLib.Items)
      {
        await NugetPackageExtractor.ExtractFileAsync(packageReader, file, destinationFolder);
      }
    }
  }

  private static async Task ExtractRuntimeFilesAsync(PackageReaderBase packageReader, string destinationFolder)
  {
    var runtimeItems = await packageReader.GetItemsAsync(PackagingConstants.Folders.Runtimes, CancellationToken.None);

    foreach (var runtimeGroup in runtimeItems)
    {
      if (!NugetRuntime.IsRuntimeCompatible(runtimeGroup.TargetFramework.GetShortFolderName()))
      {
        continue;
      }

      foreach (var file in runtimeGroup.Items)
      {
        await NugetPackageExtractor.ExtractFileAsync(packageReader, file, destinationFolder);
      }
    }
  }

  private static async Task ExtractContentFilesAsync(PackageReaderBase packageReader, string destinationFolder)
  {
    var contentItems = await packageReader.GetContentItemsAsync(CancellationToken.None);
    var compatibleContent = NugetRuntime.GetMostCompatibleGroup(contentItems.ToArray());

    if (compatibleContent != null)
    {
      foreach (var file in compatibleContent.Items)
      {
        await NugetPackageExtractor.ExtractFileAsync(packageReader, file, destinationFolder);
      }
    }
  }

  private static async Task ExtractBuildFilesAsync(PackageReaderBase packageReader, string destinationFolder)
  {
    var buildItems = await packageReader.GetItemsAsync(PackagingConstants.Folders.Build, CancellationToken.None);
    var compatibleBuild = NugetRuntime.GetMostCompatibleGroup(buildItems.ToArray());

    if (compatibleBuild != null)
    {
      var buildFolder = Path.Combine(destinationFolder, "build");
      Directory.CreateDirectory(buildFolder);

      foreach (var file in compatibleBuild.Items)
      {
        await NugetPackageExtractor.ExtractFileAsync(packageReader, file, buildFolder);
      }
    }
  }

  private static async Task ExtractFileAsync(PackageReaderBase packageReader, string file, string destinationFolder)
  {
    var fileName = Path.GetFileName(file);
    var destinationPath = Path.Combine(destinationFolder, fileName);

    await using var packageStream = await packageReader.GetStreamAsync(file, CancellationToken.None);
    await using var fileStream = File.Create(destinationPath);
    await packageStream.CopyToAsync(fileStream);
  }
}
