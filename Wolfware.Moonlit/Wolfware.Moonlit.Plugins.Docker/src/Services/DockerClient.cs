using System.Diagnostics;
using System.Text;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Docker.Abstractions;

namespace Wolfware.Moonlit.Plugins.Docker.Services;

internal sealed class DockerClient : IDockerClient
{
  private readonly ILogger<DockerClient> _logger;

  public DockerClient(ILogger<DockerClient> logger)
  {
    _logger = logger;
  }

  public Task Run(string command, string[] arguments, ReadOnlyMemory<char> stdIn,
    CancellationToken cancellationToken = default)
  {
    return this.RunCommand(command, arguments, stdIn, cancellationToken);
  }

  public Task Run(string command, string[] arguments, CancellationToken cancellationToken = default)
  {
    return this.RunCommand(command, arguments, null, cancellationToken);
  }

  private async Task RunCommand(string command, string[] arguments, ReadOnlyMemory<char>? stdIn,
    CancellationToken cancellationToken = default)
  {
    using var process = DockerClient.CreateDockerProcess(command, arguments, stdIn.HasValue, true, true);
    DockerClient.BindProcessOutput(process, this._logger);

    process.Start();
    process.BeginOutputReadLine();
    process.BeginErrorReadLine();

    if (stdIn.HasValue)
    {
      await process.StandardInput.WriteAsync(stdIn.Value, cancellationToken);
      await process.StandardInput.FlushAsync(cancellationToken);
      process.StandardInput.Close();
    }

    await process.WaitForExitAsync(cancellationToken);

    if (process.ExitCode != 0)
    {
      throw new Exception($"Docker command failed with exit code {process.ExitCode}");
    }
  }

  private static Process CreateDockerProcess(string command, string[] arguments, bool redirectInput,
    bool redirectOutput, bool redirectError)
  {
    var processStartInfo = new ProcessStartInfo
    {
      FileName = "docker",
      Arguments = $"{command} {string.Join(" ", arguments)}",
      UseShellExecute = false,
      RedirectStandardInput = redirectInput,
      RedirectStandardOutput = redirectOutput,
      RedirectStandardError = redirectError,
      CreateNoWindow = true
    };

    return new Process {StartInfo = processStartInfo};
  }

  private static void BindProcessOutput(Process process, ILogger logger)
  {
    process.OutputDataReceived += (_, e) =>
    {
      var line = e.Data;
      if (string.IsNullOrWhiteSpace(line))
      {
        return;
      }

      DockerClient.LogProcessOutput(line, logger);
    };

    process.ErrorDataReceived += (_, e) =>
    {
      var line = e.Data;
      if (string.IsNullOrWhiteSpace(line))
      {
        return;
      }

      DockerClient.LogProcessOutput(line, logger);
    };
  }

  private static void LogProcessOutput(string line, ILogger logger)
  {
    if (line.Contains("error", StringComparison.OrdinalIgnoreCase))
    {
      logger.LogError(line);
    }
    else if (line.Contains("warning", StringComparison.OrdinalIgnoreCase))
    {
      logger.LogWarning(line);
    }
    else
    {
      logger.LogInformation(line);
    }
  }
}
