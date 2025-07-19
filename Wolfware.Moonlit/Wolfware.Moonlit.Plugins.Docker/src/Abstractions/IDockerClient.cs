namespace Wolfware.Moonlit.Plugins.Docker.Abstractions;

public interface IDockerClient
{
  Task RunDockerCommand(string command, string[] arguments);
}
