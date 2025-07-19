namespace Wolfware.Moonlit.Plugins.Docker.Abstractions;

public interface IDockerClient
{
  Task Run(string command, string[] arguments, ReadOnlyMemory<char> stdIn,
    CancellationToken cancellationToken = default);

  Task Run(string command, string[] arguments, CancellationToken cancellationToken = default);
}
