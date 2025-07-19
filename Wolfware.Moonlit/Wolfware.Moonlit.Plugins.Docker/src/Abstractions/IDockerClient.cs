namespace Wolfware.Moonlit.Plugins.Docker.Abstractions;

public interface IDockerClient
{
  Task Login(string? server, string username, string password);
}
