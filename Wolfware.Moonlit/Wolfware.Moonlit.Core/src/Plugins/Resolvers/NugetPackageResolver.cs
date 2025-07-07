using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Core.Abstractions;
using Wolfware.Moonlit.Core.Nuget;

namespace Wolfware.Moonlit.Core.Plugins.Resolvers;

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
    var packageId = assemblyUri.Host;
    var version = assemblyUri.AbsolutePath.Trim('/');

    if (string.IsNullOrWhiteSpace(packageId))
    {
      throw new ArgumentException("Package ID is missing in the URI.");
    }

    if (string.IsNullOrWhiteSpace(version))
    {
      throw new ArgumentException("Package version is missing in the URI.");
    }

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
      await this._packageExtractor.ExtractPackageContentAsync(packageId, version, packagePath, cancellationToken)
        .ConfigureAwait(false);
      this._logger.LogInformation("Plugin installed successfully");
    }

    var pluginPath = Path.Combine(packagePath, $"{packageId}.dll");

    if (!File.Exists(pluginPath))
    {
      throw new FileNotFoundException($"Plugin assembly not found at path: {pluginPath}");
    }

    return pluginPath;
  }
}
