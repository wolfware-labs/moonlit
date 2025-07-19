namespace Wolfware.Moonlit.Plugins.Docker.Configuration;

public sealed class LoginConfiguration
{
  public string? Server { get; set; }

  public string Username { get; set; } = string.Empty;

  public string Password { get; set; } = string.Empty;
}
