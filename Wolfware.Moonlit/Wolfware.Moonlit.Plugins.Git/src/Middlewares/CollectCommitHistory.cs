using System.Text.RegularExpressions;
using Microsoft.Extensions.Configuration;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Git.Configuration;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Git.Middlewares;

public sealed class CollectCommitHistory : IReleaseMiddleware
{
  public Task<PipelineResult> ExecuteAsync(PipelineContext context, IConfiguration configuration)
  {
    ArgumentNullException.ThrowIfNull(context, nameof(context));
    ArgumentNullException.ThrowIfNull(configuration, nameof(configuration));

    var config = configuration.Get<CollectCommitHistoryConfiguration>();
    if (config == null)
    {
      throw new InvalidOperationException("Configuration for CollectCommitHistory is not set.");
    }

    var tagRegex = new Regex(config.TagRegex, RegexOptions.Compiled | RegexOptions.IgnoreCase);

  }
}
