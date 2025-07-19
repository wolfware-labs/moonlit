using System.Diagnostics;
using System.Text;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Docker.Abstractions;

namespace Wolfware.Moonlit.Plugins.Docker.Services;

internal sealed class DockerClient : IDockerClient
{
  public async Task RunDockerCommand(string command, string[] arguments, params KeyValuePair<string, string>[] options)
  {
    var commandOptions = options
      .Where(arg => !string.IsNullOrWhiteSpace(arg.Value))
      .Select(arg => $"{arg.Key} {arg.Value}")
      .ToArray();

    var commandArguments = string.Join(" ", arguments);

    var process = new Process
    {
      StartInfo = new ProcessStartInfo
      {
        FileName = "docker",
        Arguments = $"{command} {commandOptions} {commandArguments}",
        UseShellExecute = false,
        RedirectStandardOutput = true,
        RedirectStandardError = true,
        CreateNoWindow = true
      }
    };

    var output = new StringBuilder();
    var error = new StringBuilder();

    process.OutputDataReceived += (_, e) =>
    {
      if (e.Data != null)
      {
        Console.WriteLine(e.Data);
        output.AppendLine(e.Data);
      }
    };

    process.ErrorDataReceived += (_, e) =>
    {
      if (e.Data != null)
      {
        Console.WriteLine($"ERROR: {e.Data}");
        error.AppendLine(e.Data);
      }
    };

    process.Start();
    process.BeginOutputReadLine();
    process.BeginErrorReadLine();

    await process.WaitForExitAsync();

    if (process.ExitCode != 0)
    {
      throw new Exception($"Docker command failed with exit code {process.ExitCode}: {error}");
    }
  }
}
