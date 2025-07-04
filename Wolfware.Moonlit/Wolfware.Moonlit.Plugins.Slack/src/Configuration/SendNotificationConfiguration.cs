namespace Wolfware.Moonlit.Plugins.Slack.Configuration;

public sealed class SendNotificationConfiguration
{
  public string Channel { get; set; } = string.Empty;

  public string Message { get; set; } = string.Empty;
}
