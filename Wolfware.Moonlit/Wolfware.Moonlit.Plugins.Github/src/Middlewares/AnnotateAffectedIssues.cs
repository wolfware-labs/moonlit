using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class AnnotateAffectedIssues : IReleaseMiddleware
{
  public Task<MiddlewareResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    // This middleware is a placeholder for annotating affected issues.
    // In a real implementation, you would interact with the GitHub API to annotate issues.

    context.Logger.LogInformation("Annotating affected issues...");

    // Simulate some processing
    Task.Delay(1000, context.CancellationToken).Wait(context.CancellationToken);

    context.Logger.LogInformation("Affected issues annotated successfully.");

    return Task.FromResult(MiddlewareResult.Success(output =>
    {
      output.Add("affectedIssues", "Issue-123, Issue-456"); // Example affected issues
    }));
  }
}
