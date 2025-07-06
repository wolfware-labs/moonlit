using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using SlackNet.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipeline;
using Wolfware.Moonlit.Plugins.Slack.Middlewares;

namespace Wolfware.Moonlit.Plugins.Slack;

/// <summary>
/// Initializes and configures the Slack plugin for the release pipeline.
/// </summary>
/// <remarks>
/// This class extends the <see cref="PluginStartup"/> base class, providing the necessary logic
/// to integrate Slack-related functionality into the release pipeline's dependency injection container.
/// It configures the Slack API client and registers middleware for sending notifications.
/// </remarks>
public sealed class SlackPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    var slackApiToken = configuration.GetValue<string>("Token");
    if (string.IsNullOrWhiteSpace(slackApiToken))
    {
      throw new ArgumentException("Slack API token is required.", nameof(slackApiToken));
    }

    services.AddSlackNet(cfg => cfg.UseApiToken(slackApiToken));
  }

  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<SendNotification>("send-notification");
  }
}
