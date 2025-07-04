using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using SlackNet.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Slack.Middlewares;

namespace Wolfware.Moonlit.Plugins.Slack;

/// <summary>
/// Represents the startup configuration for the plugin.
/// Implements the <see cref="Wolfware.Moonlit.Plugins.Abstractions.IPluginStartup"/> interface
/// to configure dependencies and middleware specific to the plugin.
/// </summary>
public sealed class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    var slackApiToken = configuration.GetValue<string>("ApiToken");
    if (string.IsNullOrWhiteSpace(slackApiToken))
    {
      throw new ArgumentException("Slack API token is required.", nameof(slackApiToken));
    }

    services.AddSlackNet(cfg => cfg.UseApiToken(slackApiToken));
    services.AddMiddleware<SendNotification>("send-notification");
  }
}
