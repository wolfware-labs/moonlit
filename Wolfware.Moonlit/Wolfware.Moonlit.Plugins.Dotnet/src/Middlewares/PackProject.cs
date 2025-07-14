using System.Diagnostics;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Dotnet.Middlewares;

public sealed class PackProject : ReleaseMiddleware<PackProjectConfiguration>
{
  private readonly ILogger<PackProject> _logger;

  public PackProject(ILogger<PackProject> logger)
  {
    _logger = logger;
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, PackProjectConfiguration configuration)
  {
    var projectPath = Path.GetFullPath(configuration.Project, context.WorkingDirectory);
    if (!File.Exists(projectPath))
    {
      return Task.FromResult(MiddlewareResult.Failure($"Project file not found at path: {projectPath}"));
    }

    var outputDirectory = Path.Combine(Path.GetTempPath(), "nupkgs", Path.GetFileNameWithoutExtension(projectPath),
      DateTime.UtcNow.ToString("yyyyMMddHHmmss"));
    Directory.CreateDirectory(outputDirectory);

    var assemblyVersion = PackProject.GetAssemblyVersion(configuration);
    if (string.IsNullOrWhiteSpace(assemblyVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "AssemblyVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    var fileVersion = PackProject.GetFileVersion(configuration);
    if (string.IsNullOrWhiteSpace(fileVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "FileVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    var informationalVersion = PackProject.GetInformationalVersion(configuration);
    if (string.IsNullOrWhiteSpace(informationalVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "InformationalVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    var packageVersion = PackProject.GetPackageVersion(configuration);
    if (string.IsNullOrWhiteSpace(packageVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "PackageVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    this._logger.LogInformation(
      "Packing project {ProjectPath} with AssemblyVersion={AssemblyVersion}, FileVersion={FileVersion}, InformationalVersion={InformationalVersion}, PackageVersion={PackageVersion}",
      projectPath, assemblyVersion, fileVersion, informationalVersion, packageVersion);
    var arguments =
      $"pack \"{projectPath}\" -p:AssemblyVersion={assemblyVersion} -p:FileVersion={fileVersion} -p:InformationalVersion={informationalVersion} -p:PackageVersion={packageVersion} --output \"{outputDirectory}\"";
    var processStartInfo = new ProcessStartInfo
    {
      FileName = "dotnet",
      Arguments = arguments,
      RedirectStandardOutput = true,
      RedirectStandardError = true,
      UseShellExecute = false,
      CreateNoWindow = true,
      WorkingDirectory = context.WorkingDirectory
    };

    try
    {
      var process = new Process {StartInfo = processStartInfo};
      process.Start();
      process.WaitForExit();
      var error = process.StandardError.ReadToEnd();
      if (process.ExitCode != 0)
      {
        return Task.FromResult(MiddlewareResult.Failure($"Failed to pack project. Error: {error}"));
      }

      var nupkgFiles = Directory.GetFiles(outputDirectory, "*.nupkg");
      switch (nupkgFiles.Length)
      {
        case 0:
          return Task.FromResult(MiddlewareResult.Failure("No .nupkg files were created."));
        case 1:
          this._logger.LogInformation("Project packed successfully. Location: {PackageLocation}", nupkgFiles[0]);
          break;
        case > 1:
          this._logger.LogWarning("Multiple .nupkg files were created. Using the first one: {NupkgFile}",
            nupkgFiles[0]);
          break;
      }

      return Task.FromResult(MiddlewareResult.Success(output =>
      {
        output.Add("PackagePath", nupkgFiles[0]);
      }));
    }
    catch (Exception e)
    {
      this._logger.LogError(e, "Failed to pack project {ProjectPath}", projectPath);
      return Task.FromResult(MiddlewareResult.Failure($"Failed to pack project: {e.Message}"));
    }
  }

  private static string? GetAssemblyVersion(PackProjectConfiguration configuration)
  {
    return configuration.AssemblyVersion ?? configuration.Version?.Split('-')[0];
  }

  private static string? GetFileVersion(PackProjectConfiguration configuration)
  {
    return configuration.FileVersion ?? configuration.Version?.Split('-')[0];
  }

  private static string? GetInformationalVersion(PackProjectConfiguration configuration)
  {
    return configuration.InformationalVersion ?? configuration.Version;
  }

  private static string? GetPackageVersion(PackProjectConfiguration configuration)
  {
    return configuration.PackageVersion ?? configuration.Version?.Split('+')[0];
  }
}
