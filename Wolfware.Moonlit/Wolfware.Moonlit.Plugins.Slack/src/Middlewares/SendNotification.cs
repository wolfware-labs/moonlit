using Microsoft.Extensions.Logging;
using SlackNet;
using SlackNet.WebApi;
using Wolfware.Moonlit.Plugins.Pipelines;
using Wolfware.Moonlit.Plugins.Slack.Configuration;

namespace Wolfware.Moonlit.Plugins.Slack.Middlewares;

public sealed class SendNotification : ReleaseMiddleware<SendNotificationConfiguration>
{
  private readonly ISlackApiClient _slackApiClient;
  private readonly ILogger<SendNotification> _logger;

  public SendNotification(ISlackApiClient slackApiClient, ILogger<SendNotification> logger)
  {
    _slackApiClient = slackApiClient;
    _logger = logger;
  }

  protected override async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context,
    SendNotificationConfiguration configuration)
  {
    if (string.IsNullOrWhiteSpace(configuration.Channel))
    {
      return MiddlewareResult.Failure("No Slack channel provided for notification.");
    }

    var messageTemplate = configuration.Message;
    if (string.IsNullOrWhiteSpace(messageTemplate))
    {
      return MiddlewareResult.Failure("No message provided for Slack notification.");
    }

    try
    {
      await _slackApiClient.Chat.PostMessage(
        new Message {Channel = configuration.Channel, Text = configuration.Message});

      this._logger.LogInformation("Notification sent to Slack channel {ChannelId}.", configuration.Channel);
      return MiddlewareResult.Success();
    }
    catch (Exception ex)
    {
      return MiddlewareResult.Failure(ex.Message);
    }
  }
}
