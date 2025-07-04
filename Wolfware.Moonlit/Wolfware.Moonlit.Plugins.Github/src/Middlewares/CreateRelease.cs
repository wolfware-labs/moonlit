using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Octokit;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Github.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class CreateRelease : IReleaseMiddleware
{
  private readonly IGitHubClient _gitHubClient;

  public CreateRelease(IGitHubClient gitHubClient)
  {
    _gitHubClient = gitHubClient;
  }

  public async Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.GetRequired<CreateReleaseConfiguration>();

    if (string.IsNullOrWhiteSpace(config.Owner) || string.IsNullOrWhiteSpace(config.Repository))
    {
      return MiddlewareResult.Failure("Repository owner or name is not specified.");
    }

    if (string.IsNullOrWhiteSpace(config.Name) || string.IsNullOrWhiteSpace(config.Tag))
    {
      return MiddlewareResult.Failure("Release name or tag is not specified.");
    }

    if (string.IsNullOrWhiteSpace(config.Body))
    {
      return MiddlewareResult.Failure("Release body is not specified.");
    }

    context.Logger.LogInformation("Creating release '{ReleaseName}' with tag '{TagName}'.", config.Name, config.Tag);

    var release = new NewRelease(config.Tag)
    {
      Name = config.Name, Body = config.Body, Draft = config.Draft, Prerelease = config.PreRelease
    };

    try
    {
      var createdRelease =
        await _gitHubClient.Repository.Release.Create(config.Owner, config.Name, release);
      context.Logger.LogInformation("Release created successfully: {ReleaseUrl}", createdRelease.HtmlUrl);
      return MiddlewareResult.Success(output =>
      {
        output.Add("ReleaseUrl", createdRelease.HtmlUrl);
      });
    }
    catch (Exception)
    {
      return MiddlewareResult.Failure("Failed to create release.");
    }
  }
}
