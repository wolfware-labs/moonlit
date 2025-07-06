using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using SlackNet;
using SlackNet.WebApi;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.Slack.Configuration;

namespace Wolfware.Moonlit.Plugins.Slack.Middlewares;

public sealed class SendNotification : ReleaseMiddleware<SendNotificationConfiguration>
{
  private readonly ISlackApiClient _slackApiClient;

  public SendNotification(ISlackApiClient slackApiClient)
  {
    _slackApiClient = slackApiClient;
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

      context.Logger.LogInformation("Notification sent to Slack channel {ChannelId}.", configuration.Channel);
      return MiddlewareResult.Success();
    }
    catch (Exception ex)
    {
      return MiddlewareResult.Failure(ex.Message);
    }
  }
}
