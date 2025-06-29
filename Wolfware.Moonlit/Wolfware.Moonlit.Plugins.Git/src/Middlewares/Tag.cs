using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class Tag : IReleaseMiddleware
{
  public Task<PipelineResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    // This middleware is a placeholder for tagging the release.
    // In a real implementation, you would interact with the git repository to create a tag.

    context.Logger.LogInformation("Tagging the release...");

    // Simulate some processing
    Task.Delay(1000, context.CancellationToken).Wait(context.CancellationToken);

    context.Logger.LogInformation("Release tagged successfully.");

    return Task.FromResult(PipelineResult.Success(new Dictionary<string, object>
    {
      {"tag", "v1.0.0"} // Example tag
    }));
  }
}
