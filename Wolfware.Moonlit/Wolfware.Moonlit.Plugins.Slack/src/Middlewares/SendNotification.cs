using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using SlackNet;
using SlackNet.WebApi;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.Slack.Configuration;

namespace Wolfware.Moonlit.Plugins.Slack.Middlewares;

public sealed class SendNotification : IReleaseMiddleware
{
  private readonly ISlackApiClient _slackApiClient;

  public SendNotification(ISlackApiClient slackApiClient)
  {
    _slackApiClient = slackApiClient;
  }

  public async Task<MiddlewareResult> ExecuteAsync(ReleaseContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context);
    ArgumentNullException.ThrowIfNull(configuration);

    var config = configuration.GetRequired<SendNotificationConfiguration>();
    if (string.IsNullOrWhiteSpace(config.Channel))
    {
      return MiddlewareResult.Failure("No Slack channel provided for notification.");
    }

    var messageTemplate = config.Message;
    if (string.IsNullOrWhiteSpace(messageTemplate))
    {
      return MiddlewareResult.Failure("No message provided for Slack notification.");
    }

    // TODO: Replace with actual data from the pipeline context if needed

    try
    {
      await _slackApiClient.Chat.PostMessage(new Message {Channel = config.Channel, Text = config.Message});

      context.Logger.LogInformation("Notification sent to Slack channel {ChannelId}.", config.Channel);
      return MiddlewareResult.Success();
    }
    catch (Exception ex)
    {
      return MiddlewareResult.Failure(ex.Message);
    }
  }
}
