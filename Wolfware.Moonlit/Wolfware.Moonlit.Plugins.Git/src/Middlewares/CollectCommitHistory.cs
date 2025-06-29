using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class CollectCommitHistory : IReleaseMiddleware
{
  public Task<PipelineResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    var middlewareConfiguration = configuration.Get<CollectCommitHistoryConfiguration>();
    // This middleware is a placeholder for collecting git history.
    // In a real implementation, you would interact with the git repository here.

    context.Logger.LogInformation("Collecting git history...");

    // Simulate some processing
    Task.Delay(1000, context.CancellationToken).Wait(context.CancellationToken);

    context.Logger.LogInformation("Git history collected successfully.");

    return Task.FromResult(PipelineResult.Success());
  }
}
