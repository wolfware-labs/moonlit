﻿using System.Diagnostics;
using System.Text;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

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

    this._logger.LogInformation("Project: {ProjectPath}", Path.GetFileName(projectPath));
    this._logger.LogInformation("AssemblyVersion: {AssemblyVersion}", assemblyVersion);
    this._logger.LogInformation("FileVersion: {FileVersion}", fileVersion);
    this._logger.LogInformation("InformationalVersion: {InformationalVersion}", informationalVersion);
    this._logger.LogInformation("PackageVersion: {PackageVersion}", packageVersion);
    this._logger.LogInformation("Output Directory: {OutputDirectory}", outputDirectory);
    this._logger.LogInformation("Configuration: {Configuration}", configuration.Configuration);
    this._logger.LogInformation("NoBuild: {NoBuild}", configuration.NoBuild);
    this._logger.LogInformation("NoRestore: {NoRestore}", configuration.NoRestore);

    var argumentsBuilder = new StringBuilder($"pack \"{projectPath}\"");
    argumentsBuilder.Append($" -p:AssemblyVersion={assemblyVersion}");
    argumentsBuilder.Append($" -p:FileVersion={fileVersion}");
    argumentsBuilder.Append($" -p:InformationalVersion={informationalVersion}");
    argumentsBuilder.Append($" -p:PackageVersion={packageVersion}");
    argumentsBuilder.Append($" --output \"{outputDirectory}\"");
    argumentsBuilder.Append($" --configuration {configuration.Configuration}");

    if (configuration.NoBuild)
    {
      argumentsBuilder.Append(" --no-build");
    }

    if (configuration.NoRestore)
    {
      argumentsBuilder.Append(" --no-restore");
    }

    var processStartInfo = new ProcessStartInfo
    {
      FileName = "dotnet",
      Arguments = argumentsBuilder.ToString(),
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
      var output = process.StandardOutput.ReadToEnd();
      var outputLines = output.Split([Environment.NewLine], StringSplitOptions.RemoveEmptyEntries)
        .Select(line => line.Trim());
      foreach (var line in outputLines)
      {
        if (line.Contains("error", StringComparison.OrdinalIgnoreCase))
        {
          this._logger.LogError(line);
        }
        else if (line.Contains("warning", StringComparison.OrdinalIgnoreCase))
        {
          this._logger.LogWarning(line);
        }
        else
        {
          this._logger.LogInformation(line);
        }
      }

      if (process.ExitCode != 0)
      {
        var error = process.StandardError.ReadToEnd();
        var errorMessage = string.IsNullOrWhiteSpace(error) ? "Unknown error." : error.Trim();
        return Task.FromResult(MiddlewareResult.Failure($"Failed to pack project. Error: {errorMessage}"));
      }

      var nupkgFiles = Directory.GetFiles(outputDirectory, "*.nupkg");
      switch (nupkgFiles.Length)
      {
        case 0:
          return Task.FromResult(MiddlewareResult.Failure("No .nupkg files were created."));
        case 1:
          this._logger.LogInformation("Project packed successfully. Package: {PackageName}",
            Path.GetFileNameWithoutExtension(nupkgFiles[0]));
          break;
        case > 1:
          this._logger.LogWarning("Multiple .nupkg files were created. Using the first one: {PackageName}",
            Path.GetFileNameWithoutExtension(nupkgFiles[0]));
          break;
      }

      return Task.FromResult(MiddlewareResult.Success(output =>
      {
        output.Add("packagePath", nupkgFiles[0]);
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
