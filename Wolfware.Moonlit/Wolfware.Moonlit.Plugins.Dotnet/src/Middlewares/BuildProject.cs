using System.Diagnostics;
using System.Text;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Dotnet.Configuration;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Dotnet.Middlewares;

public sealed class BuildProject : ReleaseMiddleware<BuildProjectConfiguration>
{
  private readonly ILogger<BuildProject> _logger;

  public BuildProject(ILogger<BuildProject> logger)
  {
    _logger = logger;
  }

  protected override Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    BuildProjectConfiguration configuration)
  {
    var projectPath = Path.GetFullPath(configuration.Project, context.WorkingDirectory);
    if (!File.Exists(projectPath))
    {
      return Task.FromResult(MiddlewareResult.Failure($"Project file not found at path: {projectPath}"));
    }

    var assemblyVersion = BuildProject.GetAssemblyVersion(configuration);
    if (string.IsNullOrWhiteSpace(assemblyVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "AssemblyVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    var fileVersion = BuildProject.GetFileVersion(configuration);
    if (string.IsNullOrWhiteSpace(fileVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "FileVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    var informationalVersion = BuildProject.GetInformationalVersion(configuration);
    if (string.IsNullOrWhiteSpace(informationalVersion))
    {
      return Task.FromResult(MiddlewareResult.Failure(
        "InformationalVersion could not be determined. Please specify it in the configuration or provide a valid Version."));
    }

    this._logger.LogInformation("Project: {ProjectPath}", Path.GetFileName(projectPath));
    this._logger.LogInformation("AssemblyVersion: {AssemblyVersion}", assemblyVersion);
    this._logger.LogInformation("FileVersion: {FileVersion}", fileVersion);
    this._logger.LogInformation("InformationalVersion: {InformationalVersion}", informationalVersion);
    this._logger.LogInformation("Configuration: {Configuration}", configuration.Configuration);
    this._logger.LogInformation("NoRestore: {NoRestore}", configuration.NoRestore);

    var argumentsBuilder = new StringBuilder($"build \"{projectPath}\"");
    argumentsBuilder.Append($" -p:AssemblyVersion={assemblyVersion}");
    argumentsBuilder.Append($" -p:FileVersion={fileVersion}");
    argumentsBuilder.Append($" -p:InformationalVersion={informationalVersion}");
    argumentsBuilder.Append($" --configuration {configuration.Configuration}");

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

      if (process.ExitCode == 0)
      {
        return Task.FromResult(MiddlewareResult.Success());
      }

      var error = process.StandardError.ReadToEnd();
      var errorMessage = string.IsNullOrWhiteSpace(error) ? "Unknown error." : error.Trim();
      return Task.FromResult(MiddlewareResult.Failure($"Failed to build project. Error: {errorMessage}"));
    }
    catch (Exception e)
    {
      this._logger.LogError(e, "Failed to pack project {ProjectPath}", projectPath);
      return Task.FromResult(MiddlewareResult.Failure($"Failed to pack project: {e.Message}"));
    }
  }

  private static string? GetAssemblyVersion(BuildProjectConfiguration configuration)
  {
    return configuration.AssemblyVersion ?? configuration.Version?.Split('-')[0];
  }

  private static string? GetFileVersion(BuildProjectConfiguration configuration)
  {
    return configuration.FileVersion ?? configuration.Version?.Split('-')[0];
  }

  private static string? GetInformationalVersion(BuildProjectConfiguration configuration)
  {
    return configuration.InformationalVersion ?? configuration.Version;
  }
}
