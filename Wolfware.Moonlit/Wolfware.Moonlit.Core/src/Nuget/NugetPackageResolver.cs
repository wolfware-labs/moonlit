﻿using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.FileSystem.Abstractions;
using Wolfware.Moonlit.Core.Nuget.Abstractions;

namespace Wolfware.Moonlit.Core.Nuget;

public sealed class NugetPackageResolver : IFilePathResolver
{
  private readonly INugetPackageExtractor _packageExtractor;
  private readonly ILogger<NugetPackageResolver> _logger;

  public NugetPackageResolver(INugetPackageExtractor packageExtractor, ILogger<NugetPackageResolver> logger)
  {
    _packageExtractor = packageExtractor;
    _logger = logger;
  }

  public async ValueTask<string> ResolvePath(Uri assemblyUri, CancellationToken cancellationToken = default)
  {
    var repositoryId = assemblyUri.Host;
    var packageInfo = assemblyUri.AbsolutePath.Trim('/').Split('/', StringSplitOptions.RemoveEmptyEntries);
    if (packageInfo.Length != 2)
    {
      throw new ArgumentException(
        "Invalid package URI format. Expected format: nuget://<repository>/<packageId>/<version>");
    }

    var packageId = packageInfo[0];
    var version = packageInfo[1];

    var appDataFolder = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
    var packagePath = Path.Combine(appDataFolder, "moonlit", "plugins", packageId.ToLower(), version);

    if (!Directory.Exists(packagePath) || !File.Exists(Path.Combine(packagePath, $"{packageId}.dll")))
    {
      if (Directory.Exists(packagePath))
      {
        Directory.Delete(packagePath, true);
      }

      Directory.CreateDirectory(packagePath);
    }

    if (!Directory.EnumerateFileSystemEntries(packagePath).Any())
    {
      this._logger.LogInformation("Installing Plugin {PackageId} version {Version}", packageId, version);
      await this._packageExtractor
        .ExtractPackageContent(repositoryId, packageId, version, packagePath, cancellationToken)
        .ConfigureAwait(false);
      this._logger.LogInformation("Plugin {PackageId} installed successfully", packageId);
    }

    var pluginPath = Path.Combine(packagePath, $"{packageId}.dll");

    if (!File.Exists(pluginPath))
    {
      throw new FileNotFoundException($"Plugin assembly not found at path: {pluginPath}");
    }

    return pluginPath;
  }
}
