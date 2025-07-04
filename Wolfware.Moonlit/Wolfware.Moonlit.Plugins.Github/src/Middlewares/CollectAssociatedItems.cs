using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Github.Middlewares;

public sealed class CollectAssociatedItems : ReleaseMiddleware<CollectAssociatedItems.Configuration>
{
  public sealed class Configuration
  {
    public bool IncludePullRequests { get; set; }

    public bool IncludeIssues { get; set; }
  }

  public override Task<MiddlewareResult> ExecuteAsync(PipelineContext context, Configuration configuration)
  {
    throw new NotImplementedException();
  }
}
